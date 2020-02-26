use super::*;
use futures::future::{ok, ready};
use futures::stream::Stream;
use rusoto_s3::ListObjectsV2Output;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A stream that can list objects, and (using member functions) delete or copy listed files.
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
    /// Flatten into a stream of Objects.
    /// WARNING: discards any error during listing
    // pub fn flatten(self) -> impl TryStream<Ok = Object, Error = Error> {
    pub fn flatten(self) -> impl Stream<Item = Result<Object, Error>> {
        self.stream
            .try_filter_map(|response| {
                let r: Option<Vec<Object>> = response.contents;
                ok(r)
            })
            .map_ok(|x| stream::iter(x).map(Ok))
            .try_flatten()
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

/// List all objects with a certain prefix
pub fn s3_list_prefix<C: S3 + Clone + Send + Sync>(
    s3: C,
    bucket: String,
    prefix: String,
) -> ListObjects<C, impl Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send> {
    let bucket1 = bucket.clone();
    s3_list_objects(s3, bucket, move || ListObjectsV2Request {
        bucket: bucket1.clone(),
        prefix: Some(prefix.clone()),
        ..Default::default()
    })
}

/// List all objects given a request factory.
/// Paging is taken care of, so you need not fill in `continuation_token` in the
/// `ListObjectsV2Request`.
///
/// `bucket` is only needed for eventual further operations on `ListObjects`.
pub fn s3_list_objects<C, F>(
    s3: C,
    bucket: String,
    request_factory: F,
) -> ListObjects<C, impl Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send>
where
    C: S3 + Clone + Send + Sync,
    F: Fn() -> ListObjectsV2Request + Send + Sync + Clone,
{
    let s3_1 = s3.clone();
    let stream = futures::stream::unfold(
        // Initial state = (next continuation token, first request)
        (None, true),
        // Transformation
        //    - the stream will yield ListObjectsV2Output
        //      and stop when there is nothing left to list
        move |(cont, first)| {
            let (s3, request_factory) = (s3_1.clone(), request_factory.clone());
            async move {
                if let (&None, false) = (&cont, first) {
                    None
                } else {
                    let result = s3
                        .list_objects_v2(ListObjectsV2Request {
                            continuation_token: cont,
                            ..request_factory()
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
    use super::*;
    use crate::test::rand_string;
    #[tokio::test]
    async fn test_s3_delete_files() {
        // Minio does paging at 10'000 fles, so we need more than that.
        // It means this test will take a minutes or two.
        let s3 = testing_s3_client();
        let dir = rand_string(14);
        const N_FILES: usize = 11_000;
        let files = (0..N_FILES).map(|i| ObjectSource::Data {
            data: vec![1, 2, 3],
            key: format!("{}/{}.file", dir, i),
        });
        s3_upload_files(
            s3.clone(),
            "test-bucket".into(),
            files,
            UploadConfig::default(),
            |result| {
                if result.seq % 100 == 0 {
                    println!("{} files uploaded", result.seq);
                }
            },
            PutObjectRequest::default,
        )
        .await
        .unwrap();

        // Delete all
        s3_list_prefix(s3.clone(), "test-bucket".into(), String::new())
            .delete_all()
            .await
            .unwrap();

        // List
        let count = s3_list_prefix(s3, "test-bucket".into(), String::new())
            .flatten()
            .try_fold(0usize, |acc, _| ok(acc + 1))
            .await
            .unwrap();

        assert_eq!(count, 0);
    }
}
