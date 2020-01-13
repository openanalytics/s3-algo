//! NOTE: futures from `futures v0.1` are used internally and thus `tokio-compat` is probably
//! needed to run the futures.

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
    time::{Duration},
};
use tokio::time::delay_for;
use tokio_util::codec::{BytesCodec, FramedRead};

mod delete;
mod upload;
mod err;
mod config;

pub use delete::*;
pub use upload::*;
pub mod timeout;
pub use err::*;
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

