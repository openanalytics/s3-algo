use crate::err::Error;
use crate::timeout::*;
use bytes::BytesMut;
use futures::{future::ok, future::Future, prelude::*, stream};
use futures_retry::{FutureRetry, RetryPolicy};
use futures_stopwatch::Stopwatch;
use rusoto_core::ByteStream;
use rusoto_s3::*;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    codec::{BytesCodec, FramedRead},
    timer::Delay,
};

mod err;
pub mod timeout;
pub use err::*;
mod config;
pub use config::UploadConfig;

#[cfg(test)]
mod test;

/// S3 client for testing - assumes local minio on port 9000 and an existing credentials profile
/// called `testing`
pub fn testing_s3_client() -> S3Client {
    use rusoto_core::{credential::ProfileProvider, region::Region, HttpClient};
    let client = HttpClient::new().unwrap();
    let region = Region::Custom {
        name: "minio".to_owned(),
        endpoint: "http://localhost:9000".to_owned(),
    };

    let mut profile = ProfileProvider::new().unwrap();
    profile.set_profile("testing");

    rusoto_s3::S3Client::new_with(client, profile, region)
}

/// Just for convenience: to provide as `path_to_key` for `s3_upload_files`
pub fn strip_prefix<P: AsRef<Path>>(prefix: P) -> impl Fn(&Path) -> PathBuf {
    move |path| path.strip_prefix(prefix.as_ref()).unwrap().to_path_buf()
}

/// Upload multiple files to S3.
/// Use `s3_upload file` for each file, and additionally provides counting of uploaded files and
/// bytes through the `progress` parameter.
///
/// `path_to_key` is a function that converts a file path to the key it should have in S3.
/// The provided `strip_prefix` function should cover most use cases.
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
pub fn s3_upload_files<P, F, C, I, T, R>(
    s3: C,
    bucket: String,
    files: I,
    path_to_key: T,
    cfg: UploadConfig,
    progress: P,
    default_request: R,
) -> impl Future<Item = (), Error = Error>
where
    P: Fn(usize, UploadFileResult) -> F,
    F: IntoFuture<Item = (), Error = Error>,
    C: S3 + Clone + Send,
    I: Iterator<Item = PathBuf>,
    T: Fn(&Path) -> PathBuf,
    R: Fn() -> PutObjectRequest + Clone,
{
    let extra_copy_time_s = cfg.extra_copy_time_s;
    let extra_copy_file_time_s = cfg.extra_copy_file_time_s;
    let copy_parallelization = cfg.copy_parallelization;
    let n_retries = cfg.n_retries;

    let timeout_state = Arc::new(Mutex::new(TimeoutState::new(cfg)));

    // Make an iterator over jobs - need to make a future which fails if dir_path does not exist
    let jobs = ok(files.map(move |path| {
        let state = timeout_state.clone();
        let key = path_to_key(path.as_ref()).to_string_lossy().to_string();

        Ok(s3_upload_file(
            s3.clone(),
            bucket.clone(),
            path.clone(),
            key,
            n_retries,
            state,
            default_request.clone(),
        ))
    }));

    jobs.and_then(move |jobs|
        // Run jobs in parallel,
        //  adding eventual delays after each file upload and also at the end,
        //  and counting the progress
        stream::iter_result(jobs)
            .and_then(move |x| {
                Delay::new(Instant::now() + Duration::from_secs(extra_copy_file_time_s))
                    .map(move |_| x).map_err(|e| Error::Delay (e))
                }
            )
            .buffer_unordered(copy_parallelization)
            .zip(stream::iter_ok(0..))
            .and_then(move |(result, i)| {
                progress(i, result)
            })
            .for_each(|()| {
                Ok(())
            })
            .and_then(move |x| {
                Delay::new(Instant::now() + Duration::from_secs(extra_copy_time_s))
                    .map(move |_| x).map_err(|e| Error::Delay (e))
                }
            ))
}

#[derive(Debug, Clone, Copy)]
pub struct UploadFileResult {
    pub bytes: u64,
    pub total_time: Duration,
    pub success_time: Duration,
    pub attempts: usize,
    pub est: f64,
}

fn s3_upload_file<C, T, R>(
    s3: C,
    bucket: String,
    path: PathBuf,
    key: String,
    n_retries: usize,
    timeout: Arc<Mutex<T>>,
    default_request: R,
) -> impl Future<Item = UploadFileResult, Error = Error>
where
    C: S3 + Clone + Send,
    T: Timeout,
    R: Fn() -> PutObjectRequest + Clone,
{
    let timeout1 = timeout.clone();
    let timeout2 = timeout;
    Stopwatch::new(
        // Time the entire file upload (across all retries)
        FutureRetry::new(
            // Future factory - creates a future that reads file while uploading it
            move |attempts| {
                // variables captures are owned by this closure, but clones need be sent to further nested closures
                let (s3, bucket, key, timeout, default_request) = (
                    s3.clone(),
                    bucket.clone(),
                    key.clone(),
                    timeout1.clone(),
                    default_request.clone(),
                );

                Stopwatch::new(
                    // Time each retry - we will thus only get the time of the last, successful result (if any)
                    tokio::fs::File::open(path.clone())
                        .and_then(|file| file.metadata())
                        .and_then(|(file, metadata)| {
                            // Create stream - we need `len` (size of file) further down the stream in the
                            // S3 put_object request
                            let len = metadata.len();
                            Ok((
                                FramedRead::new(file, BytesCodec::new()).map(BytesMut::freeze),
                                len,
                            ))
                        })
                        .map_err({
                            let path = path.clone();
                            move |source| Error::Io {
                                source: source,
                                description: format!("{}", path.display()),
                            }
                        })
                        .and_then(move |(stream, len)| {
                            let (est, timeout_value) = {
                                let t = timeout.lock().unwrap();
                                (t.get_estimate(), t.get_timeout(len, attempts))
                            }; // TODO simplify this back
                            s3.put_object(PutObjectRequest {
                                bucket: bucket.clone(),
                                key: key.clone(),
                                body: Some(ByteStream::new(stream)),
                                content_length: Some(len as i64),
                                ..default_request()
                            })
                            .with_timeout(timeout_value)
                            .map_err(move |e| Error::PutObject { source: e, key })
                            .map(move |_| (est, len))
                        }),
                )
            },
            // retry function
            {
                move |e, attempts| {
                    if attempts > n_retries {
                        RetryPolicy::ForwardError(e)
                    } else {
                        RetryPolicy::WaitRetry(Duration::from_millis(200)) //  TODO adjust the time, maybe depending on retries
                    }
                }
            },
        ),
    )
    .map(
        move |((((est, bytes), success_time), attempts), total_time)| {
            // Results from this upload are in - update the timeout state
            let res = UploadFileResult {
                bytes,
                total_time,
                success_time,
                attempts,
                est,
            };
            timeout2.lock().unwrap().update(&res);
            res
        },
    )
}
