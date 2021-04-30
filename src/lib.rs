//! # S3 high-performance algorithms
//! High-performance algorithms for batch operations in Amazon S3.
//!
//! https://docs.aws.amazon.com/AmazonS3/latest/dev/optimizing-performance-guidelines.html
//!
//! - Upload multiple files with `S3Algo::upload_files`.
//! - List files with `S3Algo::s3_list_objects` or `S3Algo::s3_list_prefix`,
//! and then execute deletion or copy on all the files.

use crate::timeout::*;
use futures::{
    future::{Future, TryFutureExt},
    prelude::*,
    stream,
};
use futures_retry::{FutureRetry, RetryPolicy};
use futures_stopwatch::try_stopwatch;
use rusoto_core::{ByteStream, RusotoError};
use rusoto_s3::*;
use snafu::futures::TryFutureExt as S;
use snafu::ResultExt;
use std::{marker::Unpin, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tokio_util::codec::{BytesCodec, FramedRead};

mod config;
pub mod err;
mod list_actions;
mod upload;

pub use list_actions::*;
pub use upload::*;
pub mod timeout;
pub use config::*;
pub use err::Error;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;

#[derive(Clone)]
pub struct S3Algo<S> {
    s3: S,
    config: Config,
}
impl<S> S3Algo<S> {
    pub fn new(s3: S) -> Self {
        Self {
            s3,
            config: Config::default(),
        }
    }
    pub fn with_config(s3: S, config: Config) -> Self {
        Self { s3, config }
    }
}

/// Result of a single S3 request.
#[derive(Debug, Clone, Copy)]
pub struct RequestReport {
    /// The number of this request in a series of multiple requests (0 if not applicable)
    pub seq: usize,
    /// Size of request - in bytes or in number of objects, depending on the type of request.
    pub size: u64,
    /// The total time including all retries
    pub total_time: Duration,
    /// The time of the successful request
    pub success_time: Duration,
    /// Number of attempts. A value of `1` means no retries - success on first attempt.
    pub attempts: usize,
    /// Estimated sec/unit that was used in this request. Useful for
    /// debugging the upload algorithm and not much more.
    pub est: f64,
}

/// Issue a single S3 request, with retries and appropriate timeouts using sane defaults.
/// Basically an easier, less general version of `s3_request`.
///
/// `extra_initial_timeout`: initial timeout of request (will increase with backoff) added to
/// `cfg.base_timeout`. It can be set to 0 if the S3 operation is a small one, but if the operation
/// size depends on for example a byte count or object count, set it to something that depends on
/// that.
pub async fn s3_single_request<F, G, R>(
    future_factory: F,
    extra_initial_timeout_s: f64,
) -> Result<(RequestReport, R), Error>
where
    F: Fn() -> G + Unpin + Clone + Send + Sync + 'static,
    G: Future<Output = Result<R, Error>> + Send,
{
    // Configure a one-time Timeout that gives the desired initial_timeout_s on first try.
    // We tell `s3_request` that the request is of size `1`

    let timeout = TimeoutState::new(
        AlgorithmConfig::default(),
        SpecificTimings {
            seconds_per_unit: extra_initial_timeout_s,
            minimum_units_for_estimation: 0, // doesn't matter
        },
    );

    s3_request(
        move || {
            let factory = future_factory.clone();
            async move { Ok((factory(), 1)) }
        },
        10,
        Arc::new(Mutex::new(timeout)),
    )
    .await
}

/// Every request to S3 should be issued with `s3_request`, which puts the appropriate timeouts and
/// retries the request, as well as times it.
///
/// `future_factory` is a bit funky, being a closure that returns a future that resolves to another
/// future. We need the closure F to run the request multiple times. Its return type G is a future
/// because it might need to for example open a file using async, which might then be used in H to
/// stream from the file...
/// This is needed so that we can get e.g. the length of the file before streaming to S3.
pub(crate) async fn s3_request<F, G, H, T, R>(
    future_factory: F,
    n_retries: usize,
    timeout: Arc<Mutex<T>>,
) -> Result<(RequestReport, R), Error>
where
    F: Fn() -> G + Unpin + Clone + Send + Sync + 'static,
    G: Future<Output = Result<(H, u64), Error>> + Send,
    H: Future<Output = Result<R, Error>> + Send,
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
                        let t = timeout.lock().await;
                        (t.get_estimate(), t.get_timeout(len, attempts1))
                    };
                    try_stopwatch(
                        tokio::time::timeout(timeout_value, request)
                            .with_context(|| err::Timeout {})
                            .map(|result| result.and_then(|x| x)), // flatten the Result<Result<(), err>, timeout err>
                    )
                    .map_ok(move |(response, success_time)| (response, success_time, len, est))
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
    .map(
        move |(((response, success_time, size, est), attempts), total_time)| {
            (
                RequestReport {
                    seq: 0,
                    size,
                    total_time,
                    success_time,
                    attempts,
                    est,
                },
                response,
            )
        },
    )
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
