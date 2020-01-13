#![type_length_limit="10000000"]
use crate::timeout::*;
use futures::{
    compat::{Compat, Future01CompatExt},
    future::{
        ok,
        Future,
        TryFutureExt,
    },
    prelude::*,
    stream};
use futures_retry::{FutureRetry, RetryPolicy};
use futures_stopwatch::try_stopwatch;
use rusoto_core::ByteStream;
use rusoto_s3::*;
use snafu::{
    futures::TryFutureExt as STryFutureEXt,
    ResultExt
};
use std::{
    marker::Unpin,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::time::delay_for;
use tokio_util::codec::{BytesCodec, FramedRead};

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
    C: S3 + Clone + Send + Unpin,
    I: Iterator<Item = PathBuf>,
    T: Fn(&Path) -> PathBuf,
    R: Fn() -> PutObjectRequest + Clone + Unpin,
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

        s3_upload_file(
            s3.clone(),
            bucket.clone(),
            path,
            key,
            n_retries,
            state,
            default_request.clone(),
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
        .map(|(result, i): (Result<UploadFileResult, Error>, usize)|
            result.map(|result| (i, result)))
        .try_for_each(|(i, mut result)| {
            result.seq = i;
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

pub fn bytes05_to_04(bytes: bytes05::Bytes) -> bytes04::Bytes {
    let mut b =bytes04::Bytes::new();
    b.extend(&*bytes);
    b
}

async fn s3_upload_file<C, T, R>(
    s3: C,
    bucket: String,
    path: PathBuf,
    key: String,
    n_retries: usize,
    timeout: Arc<Mutex<T>>,
    default_request: R,
) -> Result<UploadFileResult, Error>
where
    C: S3 + Clone + Send + Unpin,
    T: Timeout,
    R: Fn() -> PutObjectRequest + Clone + Unpin,
{
    let timeout1 = timeout.clone();
    let timeout2 = timeout;
    try_stopwatch(
        // Time the entire file upload (across all retries)
        FutureRetry::new(
            // Future factory - creates a future that reads file while uploading it
            move |attempts| {
                // variables captures are owned by this closure, but clones need be sent to further nested closures
                let (s3, bucket, key, timeout, default_request, path) = (
                    s3.clone(),
                    bucket.clone(),
                    key.clone(),
                    timeout1.clone(),
                    default_request.clone(),
                    path.clone()
                );

                try_stopwatch(
                    async move {
                        // Time each retry - we will thus only get the time of the last, successful result (if any)
                        let file = tokio::fs::File::open(path.clone()).await
                            .with_context({
                                let path = path.clone();
                                move || err::Io {description: path.display().to_string()}
                            })?;
                        let metadata = file.metadata().await
                            .with_context({
                                let path = path.clone();
                                move || err::Io {description: path.display().to_string()}
                            })?;
                        // Create stream - we need `len` (size of file) further down the stream in the
                        // S3 put_object request
                        let len = metadata.len();

                        // let a : String =ok(FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze));

                        let stream = Compat::new(
                            ok(FramedRead::new(file, BytesCodec::new()).map_ok(bytes05::BytesMut::freeze))
                                .try_flatten_stream()
                                .map_ok(bytes05_to_04)
                        );
                        // ^ TODO how to remove the ok()?

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
                        .compat()
                        .with_context(move || err::PutObject { key })
                        .map_ok(move |_| (est, len))
                        .await
                    }
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
    ).await
    .map(
        move |((((est, bytes), success_time), attempts), total_time)| {
            // Results from this upload are in - update the timeout state
            let res = UploadFileResult {
                seq: 0, // will be set by the caller (s3_upload_files)!
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
