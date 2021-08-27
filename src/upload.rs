use super::*;

impl<S: S3 + Clone + Send + Sync + Unpin + 'static> S3Algo<S> {
    /// Upload multiple files to S3.
    ///
    /// `upload_files` provides counting of uploaded files and bytes through the `progress` closure:
    ///
    /// For common use cases it is adviced to use [`files_recursive`](files_recursive) for the `files` parameter.
    ///
    /// `progress` will be called after the upload of each file, with some data about that upload.
    /// The first `usize` parameter is the number of this file in the upload, while [`RequestReport`](struct.RequestReport.html)
    /// holds more data such as size in bytes, and the duration of the upload. It is thus possible to
    /// report progress both in amount of files, or amount of bytes, depending on what granularity is
    /// desired.
    /// `progress` returns a generic `F: Future` to support async operations like, for example, logging the
    /// results to a file; this future will be run as part of the upload algorithm.
    ///
    /// `default_request` constructs the default request struct - only the fields `bucket`, `key`,
    /// `body` and `content_length` are overwritten by the upload algorithm.
    pub async fn upload_files<P, F, I, R>(
        &self,
        bucket: String,
        files: I,
        progress: P,
        default_request: R,
    ) -> Result<(), Error>
    where
        P: Fn(RequestReport) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
        I: Iterator<Item = ObjectSource> + Send + 'static,
        R: Fn() -> PutObjectRequest + Clone + Unpin + Sync + Send + 'static,
    {
        let copy_parallelization = self.config.copy_parallelization;
        let n_retries = self.config.algorithm.n_retries;

        let timeout_state = Arc::new(Mutex::new(TimeoutState::new(
            self.config.algorithm.clone(),
            self.config.put_requests.clone(),
        )));
        let timeout_state2 = timeout_state.clone();

        let jobs = files.map(move |src| {
            let (default, bucket, s3) = (default_request.clone(), bucket.clone(), self.s3.clone());
            s3_request(
                move || {
                    src.clone()
                        .create_upload_future(s3.clone(), bucket.clone(), default.clone())
                },
                |_, size| size,
                n_retries,
                timeout_state.clone(),
            )
            .boxed()
        });

        // Run jobs in parallel,
        //  adding eventual delays after each file upload and also at the end,
        //  and counting the progress
        stream::iter(jobs)
            .buffer_unordered(copy_parallelization)
            .zip(stream::iter(0..))
            .map(|(result, i)| result.map(|result| (i, result)))
            .try_for_each(move |(i, (mut result, _))| {
                let progress = progress.clone();
                let timeout_state = timeout_state2.clone();
                async move {
                    result.seq = i;
                    timeout_state.lock().await.update(&result);
                    progress(result).map(Ok).await
                }
            })
            .await
    }
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
    pub async fn create_stream(&self) -> Result<(ByteStream, usize), Error> {
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

                let len = metadata.len() as usize;

                Ok((
                    ByteStream::new(
                        FramedRead::new(file, BytesCodec::new()).map_ok(bytes::BytesMut::freeze),
                    ),
                    len,
                ))
            }
            Self::Data { data, .. } => Ok((data.clone().into(), data.len())),
        }
    }
    pub async fn create_upload_future<C, R>(
        self,
        s3: C,
        bucket: String,
        default: R,
    ) -> Result<(impl Future<Output = Result<(), Error>>, usize), Error>
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
            .map_err(|e| e.into())
            .await
            .map(drop)
        };
        Ok((future, len))
    }
    pub fn get_key(&self) -> &str {
        match self {
            Self::File { key, .. } => key,
            Self::Data { key, .. } => key,
        }
    }
}

/// Convenience function (using `walkdir`) to traverse all files in directory `src_dir`. Returns an
/// iterator that can be used as input to `S3Algo::upload_files`, which uploads files
/// with a key equal to the file's path with `src_dir` stripped away, and with `key_prefix`
/// prepended.
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
