use super::*;
use futures::future::ready;

/// Delete all objects with prefix `prefix`.
///
/// Optimized by simultaneously requesting lists of files, and requesting deletion of files as
/// lists are received.
pub fn s3_delete_prefix<C: S3 + Clone + Send + 'static>(
    s3: C,
    bucket: String,
    prefix: String,
) -> impl futures::future::Future<Output = Result<(), Error>> {
    let s3_1 = s3.clone();
    let s3_2 = s3;
    let bucket1 = bucket.clone();

    futures::stream::unfold(
        // Initial state = (next continuation token, first request)
        (None, true),
        // Transformation
        //    - the stream will yield ListObjectsV2Output
        //      and stop when there is nothing left to list
        move |(cont, first)| {
            let s3 = s3_1.clone();
            let bucket = bucket1.clone();
            let prefix = Some(prefix.clone());
            async move {
                if let (&None, false) = (&cont, first) {
                    None
                } else {
                    let result = s3
                        .list_objects_v2(ListObjectsV2Request {
                            bucket,
                            prefix,
                            continuation_token: cont,
                            ..Default::default()
                        })
                        .await
                        .map_err(|e| err::Error::ListObjectsV2 { source: e });
                    let next_cont = if let Ok(ref response) = result {
                        response.next_continuation_token.clone()
                    } else {
                        None
                    };
                    Some((result, (next_cont, false)))
                }
            }
        },
    )
    // For each ListObjectsV2Output, send a request to delete all the listed objects
    .filter_map(|response| ready(response.map(|r| r.contents).transpose()))
    .try_for_each(move |contents| {
        let s3 = s3_2.clone();
        let bucket = bucket.clone();
        async move {
            s3.delete_objects(DeleteObjectsRequest {
                bucket,
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
            .map_ok(drop)
            .map_err(|e| err::Error::DeleteObjects { source: e })
            .await
        }
    })
}
