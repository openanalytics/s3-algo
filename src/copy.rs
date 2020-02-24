use super::*;
use snafu::futures::TryFutureExt;

#[derive(Clone, Debug)]
pub enum Uri {
    Local { path: PathBuf },
    Remote { bucket: String, key: String },
}

pub trait ObjectProvider {
    // provides (stream, len, key)
    type Iter: Iterator<Item=Self::Fut>;
    type Fut: Future<Output=(Self::Closure, String)>;
    type Closure: Fn() -> (ByteStream, u64) + Unpin;
    fn iter() -> Iter;
}
pub struct AllFilesProvider {
}
impl ObjectProvider for AllFilesProvider {
    type Iter = impl Iterator<Item=Self::Fut>;
    type Fut = impl Future<Output=(Self::Closure, String)>;
    type Closure = impl Fn() -> (ByteStream, u64) + Unpin;
    fn iter() -> Iter {
        walkdir::WalkDir::new(&unimplemented!())
            .into_iter()
            .filter_map(|entry|
                entry.ok().and_then(|entry| {
                    if entry.file_type().is_file() {
                        Some(entry.path().to_owned())
                    } else {
                        None
                    }
                }))
    }
    
}


/// Upload multiple files to S3.
///
/// `path_to_key` is a function that converts a file path to the key it should have in S3.
/// The provided `strip_prefix` function should cover most use cases.
///
/// `s3_upload_files` provides counting of uploaded files and bytes through the `progress` closure:
/// `progress` will be called after the upload of each file, with some data about that upload.
/// The first `usize` parameter is the number of this file in the upload, while [`UploadFileResult`](struct.UploadFileResult.html)
/// holds more data such as size in bytes, and the duration of the upload. It is thus possible to
/// report progress both in amount of files, or amount of bytes, depending on what granularity is
/// desired.
/// `progress` returns a generic `F: Future` to support async operations like, for example, logging the
/// results to a file; this future will be run as part of the upload algorithm.
///
/// `default_request` constructs the default request struct - only the fields `bucket`, `key`,
/// `body` and `content_length` are overwritten by the upload algorithm.
pub async fn s3_upload_files<P, F1, F2, C, I, T, R, G>(
    s3: C,
    bucket: String,
    files: I,
    cfg: UploadConfig,
    progress: P,
    default_request: R,
) -> Result<(), Error>
where
    C: S3 + Clone + Send + Sync + Unpin,
    P: Fn(UploadFileResult) -> F1,
    F1: Future<Output = Result<(), Error>>,
    I: Iterator<Item=F2>,
    F2: Future<Output=(G, String)>,
    G: Fn() -> (ByteStream, u64) + Unpin,
    T: Fn(&Path) -> PathBuf,
    R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send,
{
    let extra_copy_time_s = cfg.extra_copy_time_s;
    let extra_copy_file_time_s = cfg.extra_copy_file_time_s;
    let copy_parallelization = cfg.copy_parallelization;
    let n_retries = cfg.n_retries;

    let timeout_state = Arc::new(Mutex::new(TimeoutState::new(cfg)));

    let jobs = files.map(move |future| {
        let state = timeout_state.clone();
        let default = default_request.clone();
        let bucket = bucket.clone();
        let s3 = s3.clone();
        async move {
            let (get_stream, key) = future.await;
            s3_request(
                move || {
                    let (stream, len) = get_stream();
                    let (s3, bucket, key, default) = (s3.clone(), bucket.clone(), key.clone(), default.clone());
                    (async move {
                        s3.put_object(PutObjectRequest {
                            bucket: bucket.clone(),
                            key: key.clone(),
                            body: Some(ByteStream::new(stream)),
                            content_length: Some(len as i64),
                            ..default()
                        })

                        .await
                        .with_context(move || err::PutObject { key })
                        .map(drop)
                    },
                    len)
                },
                n_retries,
                state.clone(),
            ).await
        }
    });

    // Run jobs in parallel,
    //  adding eventual delays after each file upload and also at the end,
    //  and counting the progress
    stream::iter(jobs)
        .then(move |x| async move {
            delay_for(Duration::from_secs(extra_copy_file_time_s)).await;
            x
        })
        .buffer_unordered(copy_parallelization)
        .zip(stream::iter(0..))
        .map(|(result, i)| result.map(|result| (i, result)))
        .try_for_each(|(i, result)| {
            let (est, bytes, total_time, success_time, attempts) = result;
            let result = UploadFileResult {
                seq: i,
                bytes,
                total_time,
                success_time,
                attempts,
                est,
            };
            progress(result)
        })
        .then(move |x| async move {
            delay_for(Duration::from_secs(extra_copy_time_s)).await;
            x
        })
        .await
}

#[derive(Debug, Clone, Copy)]
pub struct UploadFileResult {
    /// The number of this file (how many files were already uploaded)
    pub seq: usize,
    /// Size in bytes of uploaded file
    pub bytes: u64,
    /// The total time it took to upload the file including all retries
    pub total_time: Duration,
    /// The time it took to upload the file looking at the successful request only
    pub success_time: Duration,
    /// Number of attempts. A value of `1` means no retries - success on first attempt.
    pub attempts: usize,
    /// Estimated bytes/ms upload speed at the initiation of the upload of this file. Useful for
    /// debugging the upload algorithm and not much more
    pub est: f64,
}

/// `s3_request` returns (est, bytes, total_time, success_time, attempts).
pub(crate) async fn s3_request<F, R, Ti>(
    future_factory: F,
    n_retries: usize,
    timeout: Arc<Mutex<Ti>>,
) -> Result<(f64, u64, Duration, Duration, usize), Error>
where
    F: Fn() -> (R, u64) + Unpin,
    R: Future<Output = Result<(), Error>> + Send,
    Ti: Timeout,
{
    let mut attempts1 = 0;
    let mut attempts2 = 0;
    try_stopwatch(
        // Time the entire file upload (across all retries)
        FutureRetry::new(
            // Future factory - creates a future that reads file while uploading it
            move || {
                attempts1 += 1;
                let (request, len) = future_factory();
                let (est, timeout_value) = {
                    let t = timeout.lock().unwrap();
                    (t.get_estimate(), t.get_timeout(len, attempts1))
                };
                try_stopwatch(
                    tokio::time::timeout(
                        timeout_value,
                        request
                    )
                    .with_context(|| err::Timeout {})
                    .map(|result| result.and_then(|x| x)) // flatten the Result<Result<(), err>, timeout err>
                )
                .map_ok(move |(_, success_time)| (success_time, len, est))
                .boxed()
            },
            // retry function
            {
                move |e| {
                    attempts2 += 1;
                    if attempts2 > n_retries {
                        RetryPolicy::ForwardError(e)
                    } else {
                        RetryPolicy::WaitRetry(Duration::from_millis(200)) //  TODO adjust the time, maybe depending on retries
                    }
                }
            },
        ),
    )
    .await
    .map(move |(((success_time, len, est), attempts), total_time)| {
        (est, len, total_time, success_time, attempts)
    })
    .map_err(|(err, _attempts)| err)
}

/// returns (current estimate, number of bytes uploaded)
pub(crate) async fn stream_to_s3<T, C, R>(
    s3: C,
    path: PathBuf,
    bucket: String,
    key: String,
    timeout: Arc<Mutex<T>>,
    attempts: usize,
    default_req: R,
) -> Result<(f64, u64), Error>
where
    C: S3 + Clone + Send + Unpin + Sync,
    T: Timeout + Send + Sync,
    R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send,
{
    let file = tokio::fs::File::open(path.clone()).await.with_context({
        let path = path.clone();
        move || err::Io {
            description: path.display().to_string(),
        }
    })?;
    let metadata = file.metadata().await.with_context({
        let path = path.clone();
        move || err::Io {
            description: path.display().to_string(),
        }
    })?;

    let len = metadata.len();

    let stream = ok(FramedRead::new(file, BytesCodec::new()).map_ok(bytes::BytesMut::freeze))
        .try_flatten_stream();

    let (est, timeout_value) = {
        let t = timeout.lock().unwrap();
        (t.get_estimate(), t.get_timeout(len, attempts))
    };
    tokio::time::timeout(
        timeout_value,
        s3.put_object(PutObjectRequest {
            bucket: bucket.clone(),
            key: key.clone(),
            body: Some(ByteStream::new(stream)),
            content_length: Some(len as i64),
            ..default_req()
        }),
    )
    .await
    .with_context(|| err::Timeout {})
    .and_then(move |result| result.with_context(move || err::PutObject { key }))
    .map(move |_| (est, len))
}
