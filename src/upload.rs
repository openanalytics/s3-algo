use super::*;

/// Upload multiple files to S3.
///
/// `path_to_key` is a function that converts a file path to the key it should have in S3.
/// The provided `strip_prefix` function should cover most use cases.
///
/// `s3_upload_files` provides counting of uploaded files and bytes through the `progress` closure:
///
/// For common use cases it is adviced to use `files_recursive` as `files`.
///
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
pub async fn s3_upload_files<P, F1, C, I, R>(
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
    I: Iterator<Item = ObjectSource>,
    R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send,
{
    let extra_copy_time_s = cfg.extra_copy_time_s;
    let extra_copy_file_time_s = cfg.extra_copy_file_time_s;
    let copy_parallelization = cfg.copy_parallelization;
    let n_retries = cfg.n_retries;

    let timeout_state = Arc::new(Mutex::new(TimeoutState::new(cfg)));

    let jobs = files.map(move |src| {
        let (default, bucket, s3) = (default_request.clone(), bucket.clone(), s3.clone());
        s3_request(
            move || {
                src.clone()
                    .create_upload_future(s3.clone(), bucket.clone(), default.clone())
            },
            n_retries,
            timeout_state.clone(),
        )
        .boxed()
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

#[derive(Clone, Debug)]
pub enum ObjectSource {
    File { path: PathBuf, key: String },
    Data { data: Vec<u8>, key: String },
}
impl ObjectSource {
    pub fn file(path: PathBuf, key: String) -> Self {
        Self::File { path, key }
    }
    pub fn data<D: Into<Vec<u8>>>(data: D, key: String) -> Self {
        Self::Data {
            data: data.into(),
            key,
        }
    }
    pub async fn create_stream(&self) -> Result<(ByteStream, u64), Error> {
        match self {
            Self::File { path, .. } => {
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

                Ok((
                    ByteStream::new(
                        ok(FramedRead::new(file, BytesCodec::new())
                            .map_ok(bytes::BytesMut::freeze))
                        .try_flatten_stream(),
                    ),
                    len,
                ))
            }
            Self::Data { data, .. } => Ok((data.clone().into(), data.len() as u64)),
        }
    }
    pub async fn create_upload_future<C, R>(
        self,
        s3: C,
        bucket: String,
        default: R,
    ) -> Result<(impl Future<Output = Result<(), Error>>, u64), Error>
    where
        C: S3 + Clone,
        R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send,
    {
        let (stream, len) = self.create_stream().await?;
        let key = self.get_key().to_owned();
        let (s3, bucket, default) = (s3.clone(), bucket.clone(), default.clone());
        let future = async move {
            s3.put_object(PutObjectRequest {
                bucket: bucket.clone(),
                key: key.clone(),
                body: Some(ByteStream::new(stream)),
                content_length: Some(len as i64),
                ..default()
            })
            .with_context(move || err::PutObject { key })
            .await
            .map(drop)
        };
        Ok((future, len))
    }
    pub fn get_key(&self) -> &str {
        match self {
            Self::File { key, .. } => &key,
            Self::Data { key, .. } => &key,
        }
    }
}

pub fn files_recursive(
    src_dir: PathBuf,
    key_prefix: PathBuf,
) -> impl Iterator<Item = ObjectSource> {
    walkdir::WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(move |entry| {
            let src_dir = src_dir.clone();
            let key_prefix = key_prefix.clone();
            entry.ok().and_then(move |entry| {
                if entry.file_type().is_file() {
                    let path = entry.path().to_owned();
                    let key_suffix = path.strip_prefix(&src_dir).unwrap().to_path_buf();
                    let key = key_prefix.join(&key_suffix);
                    Some(ObjectSource::File {
                        path,
                        key: key.to_string_lossy().to_string(),
                    })
                } else {
                    None
                }
            })
        })
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

#[cfg(test)]
mod test {
    use super::*;
    use tempdir::TempDir;
    #[test]
    fn test_files_recursive() {
        let tmp_dir = TempDir::new("s3-testing").unwrap();
        let dir = tmp_dir.path();
        for i in 0..10 {
            std::fs::write(dir.join(format!("img_{}.tif", i)), "file contents").unwrap();
        }
        let files = files_recursive(dir.to_owned(), PathBuf::new());
        assert_eq!(files.count(), 10);
    }
}
