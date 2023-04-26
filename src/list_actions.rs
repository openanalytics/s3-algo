use super::*;
use aws_sdk_s3::model::{Delete, Object, ObjectIdentifier};
// use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use aws_sdk_s3::output::ListObjectsV2Output;
use aws_sdk_s3::types::ByteStream;
use futures::future::{ok, ready};
use futures::stream::Stream;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io;

pub type ListObjectsV2Result = Result<(RequestReport, ListObjectsV2Output), err::Error>;

/// A stream that can list objects, and (using member functions) delete or copy listed files.
pub struct ListObjects<S> {
    s3: Client,
    config: Config,
    bucket: String,
    /// Common prefix (as requested) of the listed objects. Empty string if all objects were
    /// requestd.
    prefix: String,
    stream: S,
}
impl<S> ListObjects<S>
where
    S: Stream<Item = ListObjectsV2Result> + Sized + Send + 'static,
{
    pub fn boxed(self) -> ListObjects<Pin<Box<dyn Stream<Item = ListObjectsV2Result> + Send>>> {
        ListObjects {
            s3: self.s3,
            config: self.config,
            bucket: self.bucket,
            stream: self.stream.boxed(),
            prefix: self.prefix,
        }
    }

    /// Calls an async closure on all the individual objects of the list operation
    pub async fn process<P, F>(self, f: P) -> Result<(), Error>
    where
        P: Fn(Object) -> F + Clone,
        F: Future<Output = ()>,
    {
        let ListObjects {
            stream, prefix: _, ..
        } = self;
        stream
            .try_filter_map(|response| ok(response.1.contents))
            .map_ok(|x| stream::iter(x).map(Ok))
            .try_flatten()
            .try_for_each_concurrent(None, move |object| {
                let f = f.clone();
                async move {
                    f(object).await;
                    Ok(())
                }
            })
            .await
    }
    /// Download all listed objects - returns a stream of the contents.
    /// Used as a basis for other `download_all_*` functions.
    pub fn download_all_stream(
        self,
    ) -> impl Stream<Item = Result<(String, ByteStream, i64), Error>> {
        let ListObjects {
            s3,
            config: _,
            bucket,
            stream,
            prefix: _,
        } = self;
        stream
            .try_filter_map(|response| ok(response.1.contents))
            .map_ok(|x| stream::iter(x).map(Ok))
            .try_flatten()
            .map(|result| {
                result.and_then(|obj| {
                    let Object { key, size, .. } = obj;
                    if let Some(key) = key {
                        Ok((key, size))
                    } else {
                        Err(err::Error::MissingKeyOrSize)
                    }
                })
            })
            .and_then(move |(key, _)| {
                let (s3, bucket) = (s3.clone(), bucket.clone());

                async move {
                    let output = s3
                        .get_object()
                        .bucket(bucket.clone())
                        .key(key.clone())
                        .send()
                        .await
                        .context(err::GetObject {
                            key: key.clone(),
                            bucket,
                        })?;
                    Ok((key, output.body, output.content_length))
                }
            })
    }

    pub fn download_all_to_vec(self) -> impl Stream<Item = Result<(String, Vec<u8>), Error>> {
        self.download_all_stream()
            .and_then(|(key, body, _)| async move {
                let mut contents = vec![];
                io::copy(&mut body.into_async_read(), &mut contents)
                    .await
                    .context(err::TokioIo)?;
                Ok((key, contents))
            })
    }

    /*
    /// Download all listed objects to file system.
    /// UNIMPLEMENTED.
    pub fn download_all(self) -> impl Future<Output = Result<(), Error>> {
        // TODO use download_all_stream
        ok(unimplemented!())
    }
    */

    /// Delete all listed objects.
    /// ## The `progress` closure
    /// The second (bool) argument says whether it's about a List or Delete request:
    ///  - false: ListObjectsV2 request
    ///  - true: DeleteObjects request
    pub fn delete_all<P, F>(self, delete_progress: P) -> impl Future<Output = Result<(), Error>>
    where
        P: Fn(RequestReport) -> F + Clone + Send + Sync + 'static,
        F: Future<Output = ()> + Send + 'static,
    {
        // For each ListObjectsV2Output, send a request to delete all the listed objects
        let ListObjects {
            s3,
            config,
            bucket,
            stream,
            prefix: _,
        } = self;
        let timeout = Arc::new(Mutex::new(TimeoutState::new(
            config.algorithm.clone(),
            config.delete_requests.clone(),
        )));
        let n_retries = config.algorithm.n_retries;
        stream.try_for_each_concurrent(None, move |object| {
            let (s3, bucket, timeout, del_progress) = (
                s3.clone(),
                bucket.clone(),
                timeout.clone(),
                delete_progress.clone(),
            );
            let objects = object
                .1
                .contents
                .unwrap()
                .iter()
                .filter_map(|obj| {
                    obj.key.as_ref().map(|key| {
                        ObjectIdentifier::builder()
                            .set_key(Some(key.clone()))
                            .set_version_id(None)
                            .build()
                    })
                })
                .collect::<Vec<_>>();
            let n_objects = objects.len();
            async move {
                let (report, _) = s3_request(
                    move || {
                        let (s3, bucket, objects) = (s3.clone(), bucket.clone(), objects.clone());
                        async move {
                            let (s3, bucket, objects) =
                                (s3.clone(), bucket.clone(), objects.clone());
                            Ok((
                                async move {
                                    s3.delete_objects()
                                        .set_bucket(Some(bucket))
                                        .set_delete(Some(
                                            Delete::builder().set_objects(Some(objects)).build(),
                                        ))
                                        .send()
                                        .await
                                        .map_err(|e| e.into())
                                },
                                n_objects,
                            ))
                        }
                    },
                    |_, size| size,
                    n_retries,
                    timeout.clone(),
                )
                .await?;
                timeout.lock().await.update(&report);
                del_progress(report).await;
                Ok(())
            }
        })
    }

    /*
    /// Flatten into a stream of Objects.
    pub fn flatten(self) -> impl Stream<Item = Result<Object, Error>> {
        self.stream
            .try_filter_map(|response| ok(response.1.contents))
            .map_ok(|x| stream::iter(x).map(Ok))
            .try_flatten()
    }

    /// This function exists to provide a stream to copy all objects, for both `copy_all` and
    /// `move_all`. The `String` that is the stream's `Item` is the _source key_. An `Ok` value
    /// thus signals (relevant when used in `move_all`) that a certain key is ready for deletion.
    fn copy_all_stream<F, R>(
        self,
        dest_bucket: Option<String>,
        mapping: F,
        default_request: R,
    ) -> impl Stream<Item = Result<String, Error>>
    where
        F: Fn(&str) -> String + Clone + Send + Sync + Unpin + 'static,
        R: Fn() -> CopyObjectRequest + Clone + Unpin + Sync + Send + 'static,
    {
        let ListObjects {
            s3,
            config,
            bucket,
            stream,
            prefix: _,
        } = self;
        let timeout = Arc::new(Mutex::new(TimeoutState::new(
            config.algorithm.clone(),
            config.put_requests.clone(),
        )));
        let n_retries = config.algorithm.n_retries;
        let dest_bucket = dest_bucket.unwrap_or_else(|| bucket.clone());
        stream
            .try_filter_map(|response| ok(response.1.contents))
            .map_ok(|x| stream::iter(x).map(Ok))
            .try_flatten()
            .try_filter_map(|obj| {
                // Just filter out any object that does not have both of `key` and `size`
                let Object { key, size, .. } = obj;
                ok(key.and_then(|key| size.map(|size| (key, size))))
            })
            .and_then(move |(key, size)| {
                let (s3, timeout) = (s3.clone(), timeout.clone());
                let request = CopyObjectRequest {
                    copy_source: format!("{}/{}", bucket, key),
                    bucket: dest_bucket.clone(),
                    key: mapping(&key),
                    ..default_request()
                };
                // println!("COPY REQUEST\n{:#?}", request);
                s3_request(
                    move || {
                        let (s3, request) = (s3.clone(), request.clone());
                        async move {
                            let (s3, request) = (s3.clone(), request.clone());
                            Ok((async move{s3.copy_object(request).context(err::CopyObject).await}, size as usize))
                        }
                    },
                    |_, size| size,
                    n_retries,
                    timeout,
                )
                .map_ok(|_| key)
            })
    }

    /// Copy all listed objects, to a different S3 location as defined in `mapping` and
    /// `dest_bucket`.
    /// If `other_bucket` is not provided, copy to same bucket
    pub fn copy_all<F, R>(
        self,
        dest_bucket: Option<String>,
        mapping: F,
        default_request: R,
    ) -> impl Future<Output = Result<(), Error>>
    where
        F: Fn(&str) -> String + Clone + Send + Sync + Unpin + 'static,
        R: Fn() -> CopyObjectRequest + Clone + Unpin + Sync + Send + 'static,
    {
        self.copy_all_stream(dest_bucket, mapping, default_request)
            .try_for_each(|_| async { Ok(()) })
    }
    // TODO: Is it possible to change copy_all so that we can move_all by just chaining copy_all
    // and delete_all? Then copy_all would need to return a stream of old keys, but does that make
    // sense in general?
    // For now, this is code duplication.
    pub fn move_all<F, R>(
        self,
        dest_bucket: Option<String>,
        mapping: F,
        default_request: R,
    ) -> impl Future<Output = Result<(), Error>>
    where
        F: Fn(&str) -> String + Clone + Send + Sync + Unpin + 'static,
        R: Fn() -> CopyObjectRequest + Clone + Unpin + Sync + Send + 'static,
    {
        let src_bucket = self.bucket.clone();
        let timeout = Arc::new(Mutex::new(TimeoutState::new(
            self.config.algorithm.clone(),
            self.config.delete_requests.clone(),
        )));
        let n_retries = self.config.algorithm.n_retries;
        let s3 = self.s3.clone();
        self.copy_all_stream(dest_bucket, mapping, default_request)
            .and_then(move |src_key| {
                let delete_request = DeleteObjectRequest {
                    bucket: src_bucket.clone(),
                    key: src_key,
                    ..Default::default()
                };
                let (s3, timeout) = (s3.clone(), timeout.clone());
                s3_request(
                    move || {
                        let (s3, delete_request) = (s3.clone(), delete_request.clone());
                        async move {
                            let (s3, delete_request) = (s3.clone(), delete_request.clone());
                            Ok((
                                async move {
                                    s3.delete_object(delete_request)
                                        .context(err::DeleteObject)
                                        .await
                                },
                                1,
                            ))
                        }
                    },
                    |_, _| 1,
                    n_retries,
                    timeout,
                )
                .map_ok(drop)
                .boxed()
            })
            .try_for_each(|_| async { Ok(()) })
            .boxed()
    }
    /// Move all listed objects by substituting their common prefix with `new_prefix`.
    pub fn move_to_prefix<R>(
        self,
        dest_bucket: Option<String>,
        new_prefix: String,
        default_request: R,
    ) -> impl Future<Output = Result<(), Error>>
    where
        R: Fn() -> CopyObjectRequest + Clone + Unpin + Sync + Send + 'static,
    {
        let old_prefix = self.prefix.clone();
        let substitute_prefix =
            move |source: &str| format!("{}{}", new_prefix, source.trim_start_matches(&old_prefix));
        self.move_all(dest_bucket, substitute_prefix, default_request)
            .boxed()
    }
    */
}

impl<S> Stream for ListObjects<S>
where
    // S: Stream<Item = ListObjectsV2Result> + Sized + Send + Unpin,
    // S: Stream<Item = Result<ListObjectsV2Output, Error>> + Sized + Send,
    S: Stream<Item = ListObjectsV2Result> + Sized + Send + Unpin,
{
    type Item = ListObjectsV2Result;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.stream).poll_next(cx)
    }
}

impl S3Algo {
    /// List objects of a bucket.
    pub fn list_prefix(
        &self,
        bucket: String,
        prefix: Option<String>,
    ) -> ListObjects<
        impl Stream<Item = Result<aws_sdk_s3::output::ListObjectsV2Output, Error>> + Sized + Send,
        // impl Stream<Item = Result<ListObjectsV2Output, Error> + Sized + Send + Unpin,>
    > {
        let n_retries = self.config.algorithm.n_retries;
        let timeout = Arc::new(Mutex::new(TimeoutState::new(
            self.config.algorithm.clone(),
            self.config.delete_requests.clone(),
        )));

        let stream = self
            .s3
            .list_objects_v2()
            .bucket(bucket.clone())
            .set_prefix(prefix)
            .into_paginator()
            .send()
            // .map_ok(|output| Ok(output.contents))
            // Turn into a stream of Objects
            .map_err(|source| err::Error::ListObjectsV2 { source });
        // .try_filter_map(|output| ok(output.contents));
        // .map_ok(|output| futures::stream::iter(output.into_iter().map(|x| Ok(x))))
        // .try_flatten();

        ListObjects {
            s3: self.s3.clone(),
            config: self.config.clone(),
            stream,
            bucket,
            prefix: String::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::rand_string;
    use rusoto_s3::PutObjectRequest;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    #[tokio::test]
    async fn test_s3_delete_files_progress() {
        // Minio does paging at 10'000 fles, so we need more than that.
        // It means this test will take a minutes or two.
        let algo = S3Algo::new(testing_sdk_client().await, testing_rusoto_client());
        let dir = rand_string(14);
        let dir2 = dir.clone();
        const N_FILES: usize = 11_000;
        let files = (0..N_FILES).map(move |i| ObjectSource::Data {
            data: vec![1, 2, 3],
            key: format!("{}/{}.file", dir2, i),
        });
        algo.upload_files(
            "test-bucket".into(),
            files,
            |result| async move {
                if result.seq % 100 == 0 {
                    println!("{} files uploaded", result.seq);
                }
            },
            PutObjectRequest::default,
        )
        .await
        .unwrap();

        let listed_files = Arc::new(AtomicUsize::new(0));
        let deleted_files = Arc::new(AtomicUsize::new(0));
        let listed_files2 = listed_files.clone();
        let deleted_files2 = deleted_files.clone();

        // Do one listing only to check the exact file names
        let present = Arc::new(Mutex::new(std::collections::HashSet::new()));
        algo.list_prefix("test-bucket".into(), Some(dir.clone()))
            .process(|object| async {
                let name = object.key.unwrap_or_else(|| "NONE".to_string());
                println!("OBJ {}", name);
                present.lock().await.insert(name);
            })
            .await
            .unwrap();
        let mut present = present.lock().await;

        // All files are present
        for i in 0..N_FILES {
            let file_name = &format!("{}/{}.file", dir, i);
            assert!(present.remove(file_name));
        }

        // No unexpected filesnames.
        // Because once, it listed 11_200 files instead of 11_000
        if !present.is_empty() {
            println!("Left-over object names: {:?}", present);
            panic!("Not empty ({} files)", present.len());
        }

        // Assert that number of files is N_FILES
        let count = algo
            .list_prefix("test-bucket".into(), Some(dir.clone()))
            .try_fold(0usize, |acc, _| ok(acc + 1))
            .await
            .unwrap();
        assert_eq!(count, N_FILES);

        // Delete all
        algo.list_prefix("test-bucket".into(), Some(dir.clone()))
            .delete_all(
                move |list_rep| {
                    let n = list_rep.size as usize;
                    println!("Listed {} items", n);
                    let listed_files = listed_files2.clone();
                    async move {
                        listed_files.fetch_add(n, Ordering::Relaxed);
                    }
                },
                move |del_rep| {
                    let n = del_rep.size as usize;
                    println!("Deleted {} items", n);
                    let deleted_files = deleted_files2.clone();
                    async move {
                        deleted_files.fetch_add(n, Ordering::Relaxed);
                    }
                },
            )
            .await
            .unwrap();

        // Assert number of objects listed and deleted
        assert_eq!(listed_files.load(Ordering::Relaxed), N_FILES);
        assert_eq!(deleted_files.load(Ordering::Relaxed), N_FILES);

        // Assert that number of files is 0
        let count = algo
            .list_prefix("test-bucket".into(), dir)
            .flatten()
            .try_fold(0usize, |acc, _| ok(acc + 1))
            .await
            .unwrap();

        assert_eq!(count, 0);
    }
}
