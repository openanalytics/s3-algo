use super::*;
use futures01::{
    future::Future,
    stream::Stream,
};

/// Delete all objects with prefix `prefix`.
///
/// Optimized by simultaneously requesting lists of files, and requesting deletion of files as
/// lists are received.
///
/// NOTE: Uses futures 0.1 internally and must thus be run with for example `tokio-compat`
pub fn s3_delete_prefix<C: S3 + Clone + Send + 'static>(
    s3: C,
    bucket: String,
    prefix: String,
) -> impl futures::future::Future<Output = Result<(), Error>> {
    let s3_1 = s3.clone();
    let s3_2 = s3;
    let bucket1 = bucket.clone();

    futures01::stream::unfold(
        // Initial state
        (None, true),
        // Transformation
        //    - the stream will yield ListObjectsV2Output
        move |(cont, first)| {
            if let (&None, false) = (&cont, first) {
                None
            } else {
                Some(
                    s3_1.list_objects_v2(ListObjectsV2Request {
                        bucket: bucket1.clone(),
                        prefix: Some(prefix.clone()),
                        continuation_token: cont,
                        ..Default::default()
                    })
                    .map(|response| (response.clone(), (response.next_continuation_token, false)))
                    .map_err(|e| err::Error::ListObjectsV2 { source: e })
                )
            }
        },
    )
    // For each ListObjectsV2Output, send a request to delete all the listed objects
    .filter_map(|response| response.contents)
    .for_each(move |contents| {
        s3_2.delete_objects(DeleteObjectsRequest {
            bucket: bucket.clone(),
            delete: Delete {
                objects: contents
                    .iter()
                    .filter_map(|obj| {
                        obj.key.as_ref().map(|key| ObjectIdentifier {
                            key: key.clone(),
                            version_id: None,
                        })
                    })
                    .collect::<Vec<_>>(),
                quiet: None,
            },
            ..Default::default()
        })
        .map(drop)
        .map_err(|e| err::Error::DeleteObjects { source: e })
    }).compat()
}
