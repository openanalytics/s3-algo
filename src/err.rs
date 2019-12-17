use snafu::{Backtrace, Snafu};
use std::{io, path::PathBuf};
type PutError = rusoto_core::RusotoError<rusoto_s3::PutObjectError>;

#[derive(Snafu, Debug)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Io error: {}: {}", description, source))]
    Io {
        source: io::Error,
        description: String,
        backtrace: Backtrace,
    },
    /// Error originating from tokio::Delay
    #[snafu(display("Tokio timer error: {}", source))]
    Delay {
        source: tokio::timer::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("S3 'put object' error on key '{}': {}", key, source))]
    PutObject {
        source: PutError,
        key: String,
        backtrace: Backtrace,
    },
}
