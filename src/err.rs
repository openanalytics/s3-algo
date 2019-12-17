use std::{io, path::PathBuf};
type PutError = rusoto_core::RusotoError<rusoto_s3::PutObjectError>;

#[derive(Debug)]
pub enum Error {
    Io {
        source: io::Error,
        description: String,
    },
    /// Error originating from tokio::Delay
    Delay(tokio::timer::Error),
    PutObject {
        source: PutError,
        key: String,
    },
    /// Conversion path->key failed
    PathToKey,
}
