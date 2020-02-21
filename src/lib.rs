//! # S3 high-performance algorithms
//! High-performance algorithms for batch operations in Amazon S3.
//!
//! https://docs.aws.amazon.com/AmazonS3/latest/dev/optimizing-performance-guidelines.html
//!
//! Per now, uploading multiple files has been the main focus.
//! Deletion of prefix is also implemented.
//! Listing of files is planned.
//!
//! NOTE: futures from `futures v0.1` are used internally and thus for example `tokio-compat` is
//! required to run the futures, until `rusoto` updates to use `tokio 0.2`.
//!
//! NOTE: also due to `rusoto` currently using `bytes 0.4`, a quick compatibility function between
//! `bytes 0.4` and `bytes 0.5` was made that is not zero-cost (clones the bytes), which will affect
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
mod delete;
mod err;
mod upload;

pub use delete::*;
pub use upload::*;
pub mod timeout;
pub use config::UploadConfig;
pub use err::*;

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
