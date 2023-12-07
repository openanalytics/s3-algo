use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::copy_object::CopyObjectError;
use aws_sdk_s3::operation::delete_object::DeleteObjectError;
use aws_sdk_s3::operation::delete_objects::DeleteObjectsError;
use aws_sdk_s3::operation::get_object::GetObjectError;
use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Error;
use aws_sdk_s3::operation::put_object::PutObjectError;
use aws_sdk_s3::primitives::ByteStreamError;
use snafu::{Backtrace, Snafu};
use std::io;

#[derive(Snafu, Debug)]
#[snafu(visibility = "pub")]
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
        source: tokio::time::error::Error,
        backtrace: Backtrace,
    },
    #[snafu(display("S3 operation timed out"))]
    Timeout {
        source: tokio::time::error::Elapsed,
    },
    #[snafu(display("Error listing objects in S3: {:?}", source))]
    ListObjectsV2 {
        source: SdkError<ListObjectsV2Error>,
    },
    #[snafu(display("Error deleting objects in S3: {:?}", source))]
    DeleteObjects {
        source: SdkError<DeleteObjectsError>,
    },
    DeleteObject {
        source: SdkError<DeleteObjectError>,
    },
    CopyObject {
        source: SdkError<CopyObjectError>,
    },
    #[snafu(display("GetObject s3://{}/{}: {:#?}", bucket, key, source))]
    GetObject {
        key: String,
        bucket: String,
        source: SdkError<GetObjectError>,
    },
    #[snafu(display("IO error: {}", source))]
    TokioIo {
        source: tokio::io::Error,
    },
    AnyError {
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[snafu(display("Downloading objects: missing key or size property"))]
    MissingKeyOrSize,
    #[snafu(display("Downloading objects: missing content_length property"))]
    MissingContentLength,

    // AWS SDK Errors
    #[snafu(display("S3 'put object' error on key '{}': {}", key, source))]
    PutObject {
        source: SdkError<PutObjectError>,
        key: String,
        backtrace: Backtrace,
    },

    #[snafu(display("Error listing objects in S3: {:?}", source))]
    NewListObjectsV2 {
        source: SdkError<ListObjectsV2Error>,
    },

    #[snafu(display("Error deleting objects in S3: {:?}", source))]
    NewDeleteObjects {
        source: SdkError<DeleteObjectsError>,
    },
    NewDeleteObject {
        source: SdkError<DeleteObjectError>,
    },
    NewCopyObject {
        source: SdkError<CopyObjectError>,
    },
    #[snafu(display("GetObject s3://{}/{}: {:#?}", bucket, key, source))]
    NewGetObject {
        key: String,
        bucket: String,
        source: SdkError<GetObjectError>,
    },
}

impl<T> From<SdkError<T>> for Error
where
    T: std::error::Error + Send + Sync + 'static,
{
    fn from(err: SdkError<T>) -> Self {
        Self::AnyError {
            source: Box::new(err),
        }
    }
}

impl From<ByteStreamError> for Error {
    fn from(err: ByteStreamError) -> Self {
        Self::AnyError {
            source: Box::new(err),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use snafu::GenerateBacktrace;
    #[test]
    fn error_traits() {
        fn foo<T: Send>(_: T) {}
        foo(Error::Io {
            source: io::Error::from_raw_os_error(1),
            description: "hello".into(),
            backtrace: Backtrace::generate(),
        });
    }
}
