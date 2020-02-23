use super::*;

#[derive(Clone, Debug)]
pub enum Uri {
    Local { path: PathBuf },
    Remote { bucket: String, key: String },
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
pub async fn s3_upload_files<P, F, C, I, T, R>(
    s3: C,
    bucket: String,
    files: I,
    path_to_key: T,
    cfg: UploadConfig,
    progress: P,
    default_request: R,
) -> Result<(), Error>
where
    P: Fn(UploadFileResult) -> F,
    F: Future<Output = Result<(), Error>>,
    C: S3 + Clone + Send + Sync + Unpin,
    I: Iterator<Item = PathBuf>,
    T: Fn(&Path) -> PathBuf,
    R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send,
{
    let extra_copy_time_s = cfg.extra_copy_time_s;
    let extra_copy_file_time_s = cfg.extra_copy_file_time_s;
    let copy_parallelization = cfg.copy_parallelization;
    let n_retries = cfg.n_retries;

    let timeout_state = Arc::new(Mutex::new(TimeoutState::new(cfg)));

    // Make an iterator over jobs - need to make a future which fails if dir_path does not exist
    let jobs = files.map(move |path| {
        let state = timeout_state.clone();
        let key = path_to_key(path.as_ref()).to_string_lossy().to_string();
        let default = default_request.clone();
        let bucket = bucket.clone();
        let s3 = s3.clone();

        s3_request(
            move |attempts| {
                stream_to_s3(
                    s3.clone(),
                    path.clone(),
                    bucket.clone(),
                    key.clone(),
                    state.clone(),
                    attempts,
                    default.clone(),
                )
            },
            n_retries,
        )
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
            let ((est, bytes), total_time, success_time, attempts) = result;
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

/// The future factory takes one argument: number of attempts so far.
/// `s3_request` returns (T, total_time, success_time, attempts).
pub(crate) async fn s3_request<T, F, R>(
    future_factory: F,
    n_retries: usize,
) -> Result<(T, Duration, Duration, usize), Error>
where
    F: Fn(usize) -> R + Unpin,
    R: Future<Output = Result<T, Error>> + Send,
{
    let mut attempts1 = 0;
    let mut attempts2 = 0;
    try_stopwatch(
        // Time the entire file upload (across all retries)
        FutureRetry::new(
            // Future factory - creates a future that reads file while uploading it
            move || {
                attempts1 += 1;
                // variables captures are owned by this closure, but clones need be sent to further nested closures
                try_stopwatch(future_factory(attempts1)).boxed() // to avoid too big type length while compiling...
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
    .map(move |(((result, success_time), attempts), total_time)| {
        (result, total_time, success_time, attempts)
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
