//! # S3 high-performance algorithms
//! High-performance algorithms for batch operations in Amazon S3.
//!
//! https://docs.aws.amazon.com/AmazonS3/latest/dev/optimizing-performance-guidelines.html
//!
//! Until now, uploading multiple files has been the main focus.
//! Deletion of prefix is also implemented.
//! Listing of files is planned.
//!
//! performance of the upload functionality, until `rusoto` updates to `tokio 0.2` and `bytes 0.5`.

use crate::timeout::*;
use futures::{
    future::{ok, Future, TryFutureExt},
    prelude::*,
    stream,
};
use futures_retry::{FutureRetry, RetryPolicy};
use futures_stopwatch::try_stopwatch;
use rusoto_core::ByteStream;
use rusoto_s3::*;
use snafu::futures::TryFutureExt as S;
use snafu::ResultExt;
use std::{
    marker::Unpin,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::delay_for;
use tokio_util::codec::{BytesCodec, FramedRead};

mod config;
mod copy;
mod err;
mod list_actions;

pub use copy::*;
pub use list_actions::*;
pub mod timeout;
pub use config::UploadConfig;
pub use err::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

/// `s3_request` returns (est, bytes, total_time, success_time, attempts).
/// `future_factory` is a bit funky, being a closure that returns a future that resolves to another
/// future. We need the closure F to run the request multiple times. Its return type G is a future
/// because it might need to for example open a file using async, which might then be used in H to
/// stream from the file...
/// This is needed so that we can get the length of the file before streaming to S3.
pub(crate) async fn s3_request<F, G, H, T>(
    future_factory: F,
    n_retries: usize,
    timeout: Arc<Mutex<T>>,
) -> Result<(f64, u64, Duration, Duration, usize), Error>
where
    F: Fn() -> G + Unpin + Clone,
    G: Future<Output = Result<(H, u64), Error>> + Send,
    H: Future<Output = Result<(), Error>> + Send,
    T: timeout::Timeout,
{
    let mut attempts1 = 0;
    let mut attempts2 = 0;
    try_stopwatch(
        // Time the entire file upload (across all retries)
        FutureRetry::new(
            // Future factory - creates a future that reads file while uploading it
            move || {
                let future_factory = future_factory.clone();
                let timeout = timeout.clone();
                async move {
                    attempts1 += 1;
                    let (request, len) = future_factory().await?;
                    let (est, timeout_value) = {
                        let t = timeout.lock().unwrap();
                        (t.get_estimate(), t.get_timeout(len, attempts1))
                    };
                    try_stopwatch(
                        tokio::time::timeout(timeout_value, request)
                            .with_context(|| err::Timeout {})
                            .map(|result| result.and_then(|x| x)), // flatten the Result<Result<(), err>, timeout err>
                    )
                    .map_ok(move |(_, success_time)| (success_time, len, est))
                    .await
                }
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
