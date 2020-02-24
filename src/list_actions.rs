use super::*;
use futures::future::ready;
use futures::stream::Stream;
use rusoto_s3::ListObjectsV2Output;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ListObjects<C, S> {
    s3: C,
    bucket: String,
    stream: S,
}
impl<C, S> ListObjects<C, S>
where
    C: S3 + Clone + Send,
    S: Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send,
{
    pub fn delete_all(self) -> impl Future<Output = Result<(), Error>> {
        // For each ListObjectsV2Output, send a request to delete all the listed objects
        let ListObjects { s3, bucket, stream } = self;
        stream
            .filter_map(|response| ready(response.map(|r| r.contents).transpose()))
            .try_for_each(move |contents| {
                let s3 = s3.clone();
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

    /*
    /// Copy all listed objects, to different bucket and keys as defined in `mapping`
    pub fn copy_all<F>(self, mapping: F) -> impl Future<Output = Result<(), Error>> {
        self.stream
            .filter_map(|response| ready(response.map(|r| r.contents).transpose()))
            .try_for_each(move |contents| {
            })
    }
    */
}

impl<C, S> Stream for ListObjects<C, S>
where
    S: Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send + Unpin,
    C: Unpin,
{
    type Item = Result<ListObjectsV2Output, Error>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

/// Generates a stream of ListObjects with all objects in a bucket with the given prefix
pub fn s3_list_objects<C: S3 + Clone + Send + Sync>(
    s3: C,
    bucket: String,
    prefix: String,
) -> ListObjects<C, impl Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send> {
    let s3_1 = s3.clone();
    let bucket1 = bucket.clone();
    let stream = futures::stream::unfold(
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
    );
    ListObjects { s3, stream, bucket }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_delete_several_pages() {
        use crate::s3_upload_files;
        // Minio uses paging only per 10'000 objects
    }
}
