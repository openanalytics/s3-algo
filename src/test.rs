use crate::testing_s3_client;
use crate::{timeout::Timeout, *};
use multi_default_trait_impl::{default_trait_impl, trait_impl};
use rand::Rng;
use rusoto_core::*;
use rusoto_s3::*;
use std::{
    io::Read,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tempdir::TempDir;

fn rand_string(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(n)
        .collect::<String>()
}

#[test]
fn s3_upload_file_attempts_count() {
    struct TimeoutState;
    impl Timeout for TimeoutState {
        fn get_timeout(&self, _bytes: u64, _attempts: usize) -> Duration {
            Duration::from_secs(4)
        }
        fn update(&mut self, _: &UploadFileResult) {}
        fn get_estimate(&self) -> f64 {
            0.0
        }
    }
    let timeout = Arc::new(Mutex::new(TimeoutState));

    const ATTEMPTS: usize = 4;
    let tmp_dir = TempDir::new("s3-testing").unwrap();

    let path = tmp_dir.path().join("file.txt");
    std::fs::write(&path, "hello".as_bytes()).unwrap();

    let cli = S3MockRetry::new(ATTEMPTS - 1);

    let future = s3_upload_file(
        cli,
        "any-bucket".into(),
        path,
        "any-key".into(),
        10,
        timeout,
        PutObjectRequest::default,
    );

    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(future).unwrap();
    assert_eq!(result.attempts, ATTEMPTS);
}

macro_rules! all_file_paths {
    ($dir_path:expr $(, max_open = $max_open:expr)?) => {
        walkdir::WalkDir::new(&$dir_path)
            $(.max_open($max_open))?
            .into_iter()
            .filter_map(|entry|
                entry.ok().and_then(|entry| {
                    if entry.file_type().is_file() {
                        Some(entry.path().to_owned())
                    } else {
                        None
                    }
                }))};
}
#[test]
fn test_s3_upload_files() {
    const N_FILES: usize = 100;
    let tmp_dir = TempDir::new("s3-testing").unwrap();
    let cfg = UploadConfig::default();

    let dir_key = Path::new(&rand_string(4))
        .join(rand_string(4))
        .join(rand_string(4));
    let dir = tmp_dir.path().join(&dir_key);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..N_FILES {
        std::fs::write(dir.join(format!("img_{}.tif", i)), "file contents").unwrap();
    }

    println!("Upload {} to {:?} ", dir.display(), dir_key);
    let future = s3_upload_files(
        testing_s3_client(),
        "test-bucket".to_string(),
        all_file_paths!(dir),
        strip_prefix(tmp_dir.path().to_owned()),
        cfg,
        |_, _res| Ok(()),
        PutObjectRequest::default,
    );

    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(future).unwrap();

    let s3 = testing_s3_client();

    // Check that all files are there
    for i in 0..N_FILES {
        // let key = format!("{}/img_{}.tif", dir_key, i);
        let key = dir_key.join(format!("img_{}.tif", i));

        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket".to_string(),
            key: key.to_str().unwrap().to_string(),
            ..Default::default()
        });
        let response = runtime.block_on(response).unwrap();

        let mut body = response.body.unwrap().into_blocking_read();
        let mut content = Vec::new();
        body.read_to_end(&mut content).unwrap();
        let content = std::str::from_utf8(&content).unwrap();
        assert_eq!(content, "file contents");
    }
}

/* TODO https://github.com/rusoto/rusoto/issues/1546
#[test]
fn test_s3_upload_file_timeout() {
    let dir = tempdir::TempDir::new("testing").unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(&path, "file contents");

    let timeout = Arc::new(Mutex::new(TimeoutState::new(UploadConfig {
        timeout_fraction: 1.0,
        backoff: 1.0,
        n_retries: 8,
        expected_upload_speed: 1.0,
        avg_power: 0.7,
        avg_min_bytes: 1_000_000,
        min_timeout: 0.5,
    })));

    let cli = S3MockTimeout::new(1);
    let future = s3_upload_file(
        cli.clone(),
        String::new(),
        path,
        String::new(),
        true,
        1,
        timeout.clone());

    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(future).unwrap();
    println!("{:?}", result);
}

#[test]
fn test_rusoto_timeout() {
    let dir = tempdir::TempDir::new("testing").unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(&path, "file contents").unwrap();

    let s3 = S3MockTimeout::new(1);


    let future =
    tokio::fs::File::open(path.clone())
        .and_then(|file| file.metadata())
        .and_then(|(file, metadata)| {
            // Create stream - we need `len` (size of file) further down the stream in the
            // S3 put_object request
            let len = metadata.len();
            Ok((FramedRead::new(file, BytesCodec::new()).map(BytesMut::freeze), len))
        })
        .map_err(|e| format!("Cannot read file: {}", e))
        .and_then(move |(stream, len)|
            s3.put_object(PutObjectRequest {
                bucket: String::new(),
                key: String::new(),
                body: Some(ByteStream::new(stream)),
                content_length: Some(len as i64),
                .. Default::default()})
                .with_timeout(Duration::from_millis(500))
                .map_err(move |e| format!("Error in put_object: {}", e))
                .map(move |_| len)
            );

    let start = Instant::now();
    let mut runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(future).unwrap();
    println!("Completed in {:?}", Instant::now() - start);
}
*/

// Just to be able to easily create mock objects, we create a temporary trait with defaults that
// panic
#[default_trait_impl]
impl S3 for S3WithDefaults {
    fn abort_multipart_upload(
        &self,
        input: AbortMultipartUploadRequest,
    ) -> RusotoFuture<AbortMultipartUploadOutput, AbortMultipartUploadError> {
        unimplemented!()
    }

    fn complete_multipart_upload(
        &self,
        input: CompleteMultipartUploadRequest,
    ) -> RusotoFuture<CompleteMultipartUploadOutput, CompleteMultipartUploadError> {
        unimplemented!()
    }

    fn copy_object(
        &self,
        input: CopyObjectRequest,
    ) -> RusotoFuture<CopyObjectOutput, CopyObjectError> {
        unimplemented!()
    }

    fn create_bucket(
        &self,
        input: CreateBucketRequest,
    ) -> RusotoFuture<CreateBucketOutput, CreateBucketError> {
        unimplemented!()
    }

    fn create_multipart_upload(
        &self,
        input: CreateMultipartUploadRequest,
    ) -> RusotoFuture<CreateMultipartUploadOutput, CreateMultipartUploadError> {
        unimplemented!()
    }

    fn delete_bucket(&self, input: DeleteBucketRequest) -> RusotoFuture<(), DeleteBucketError> {
        unimplemented!()
    }

    fn delete_bucket_analytics_configuration(
        &self,
        input: DeleteBucketAnalyticsConfigurationRequest,
    ) -> RusotoFuture<(), DeleteBucketAnalyticsConfigurationError> {
        unimplemented!()
    }

    fn delete_bucket_cors(
        &self,
        input: DeleteBucketCorsRequest,
    ) -> RusotoFuture<(), DeleteBucketCorsError> {
        unimplemented!()
    }

    fn delete_bucket_encryption(
        &self,
        input: DeleteBucketEncryptionRequest,
    ) -> RusotoFuture<(), DeleteBucketEncryptionError> {
        unimplemented!()
    }

    fn delete_bucket_inventory_configuration(
        &self,
        input: DeleteBucketInventoryConfigurationRequest,
    ) -> RusotoFuture<(), DeleteBucketInventoryConfigurationError> {
        unimplemented!()
    }

    fn delete_bucket_lifecycle(
        &self,
        input: DeleteBucketLifecycleRequest,
    ) -> RusotoFuture<(), DeleteBucketLifecycleError> {
        unimplemented!()
    }

    fn delete_bucket_metrics_configuration(
        &self,
        input: DeleteBucketMetricsConfigurationRequest,
    ) -> RusotoFuture<(), DeleteBucketMetricsConfigurationError> {
        unimplemented!()
    }

    fn delete_bucket_policy(
        &self,
        input: DeleteBucketPolicyRequest,
    ) -> RusotoFuture<(), DeleteBucketPolicyError> {
        unimplemented!()
    }

    fn delete_bucket_replication(
        &self,
        input: DeleteBucketReplicationRequest,
    ) -> RusotoFuture<(), DeleteBucketReplicationError> {
        unimplemented!()
    }

    fn delete_bucket_tagging(
        &self,
        input: DeleteBucketTaggingRequest,
    ) -> RusotoFuture<(), DeleteBucketTaggingError> {
        unimplemented!()
    }

    fn delete_bucket_website(
        &self,
        input: DeleteBucketWebsiteRequest,
    ) -> RusotoFuture<(), DeleteBucketWebsiteError> {
        unimplemented!()
    }

    fn delete_object(
        &self,
        input: DeleteObjectRequest,
    ) -> RusotoFuture<DeleteObjectOutput, DeleteObjectError> {
        unimplemented!()
    }

    fn delete_object_tagging(
        &self,
        input: DeleteObjectTaggingRequest,
    ) -> RusotoFuture<DeleteObjectTaggingOutput, DeleteObjectTaggingError> {
        unimplemented!()
    }

    fn delete_objects(
        &self,
        input: DeleteObjectsRequest,
    ) -> RusotoFuture<DeleteObjectsOutput, DeleteObjectsError> {
        unimplemented!()
    }

    fn delete_public_access_block(
        &self,
        input: DeletePublicAccessBlockRequest,
    ) -> RusotoFuture<(), DeletePublicAccessBlockError> {
        unimplemented!()
    }

    fn get_bucket_accelerate_configuration(
        &self,
        input: GetBucketAccelerateConfigurationRequest,
    ) -> RusotoFuture<GetBucketAccelerateConfigurationOutput, GetBucketAccelerateConfigurationError>
    {
        unimplemented!()
    }

    fn get_bucket_acl(
        &self,
        input: GetBucketAclRequest,
    ) -> RusotoFuture<GetBucketAclOutput, GetBucketAclError> {
        unimplemented!()
    }

    fn get_bucket_analytics_configuration(
        &self,
        input: GetBucketAnalyticsConfigurationRequest,
    ) -> RusotoFuture<GetBucketAnalyticsConfigurationOutput, GetBucketAnalyticsConfigurationError>
    {
        unimplemented!()
    }

    fn get_bucket_cors(
        &self,
        input: GetBucketCorsRequest,
    ) -> RusotoFuture<GetBucketCorsOutput, GetBucketCorsError> {
        unimplemented!()
    }

    fn get_bucket_encryption(
        &self,
        input: GetBucketEncryptionRequest,
    ) -> RusotoFuture<GetBucketEncryptionOutput, GetBucketEncryptionError> {
        unimplemented!()
    }

    fn get_bucket_inventory_configuration(
        &self,
        input: GetBucketInventoryConfigurationRequest,
    ) -> RusotoFuture<GetBucketInventoryConfigurationOutput, GetBucketInventoryConfigurationError>
    {
        unimplemented!()
    }

    fn get_bucket_lifecycle(
        &self,
        input: GetBucketLifecycleRequest,
    ) -> RusotoFuture<GetBucketLifecycleOutput, GetBucketLifecycleError> {
        unimplemented!()
    }

    fn get_bucket_lifecycle_configuration(
        &self,
        input: GetBucketLifecycleConfigurationRequest,
    ) -> RusotoFuture<GetBucketLifecycleConfigurationOutput, GetBucketLifecycleConfigurationError>
    {
        unimplemented!()
    }

    fn get_bucket_location(
        &self,
        input: GetBucketLocationRequest,
    ) -> RusotoFuture<GetBucketLocationOutput, GetBucketLocationError> {
        unimplemented!()
    }

    fn get_bucket_logging(
        &self,
        input: GetBucketLoggingRequest,
    ) -> RusotoFuture<GetBucketLoggingOutput, GetBucketLoggingError> {
        unimplemented!()
    }

    fn get_bucket_metrics_configuration(
        &self,
        input: GetBucketMetricsConfigurationRequest,
    ) -> RusotoFuture<GetBucketMetricsConfigurationOutput, GetBucketMetricsConfigurationError> {
        unimplemented!()
    }

    fn get_bucket_notification(
        &self,
        input: GetBucketNotificationConfigurationRequest,
    ) -> RusotoFuture<NotificationConfigurationDeprecated, GetBucketNotificationError> {
        unimplemented!()
    }

    fn get_bucket_notification_configuration(
        &self,
        input: GetBucketNotificationConfigurationRequest,
    ) -> RusotoFuture<NotificationConfiguration, GetBucketNotificationConfigurationError> {
        unimplemented!()
    }

    fn get_bucket_policy(
        &self,
        input: GetBucketPolicyRequest,
    ) -> RusotoFuture<GetBucketPolicyOutput, GetBucketPolicyError> {
        unimplemented!()
    }

    fn get_bucket_policy_status(
        &self,
        input: GetBucketPolicyStatusRequest,
    ) -> RusotoFuture<GetBucketPolicyStatusOutput, GetBucketPolicyStatusError> {
        unimplemented!()
    }

    fn get_bucket_replication(
        &self,
        input: GetBucketReplicationRequest,
    ) -> RusotoFuture<GetBucketReplicationOutput, GetBucketReplicationError> {
        unimplemented!()
    }

    fn get_bucket_request_payment(
        &self,
        input: GetBucketRequestPaymentRequest,
    ) -> RusotoFuture<GetBucketRequestPaymentOutput, GetBucketRequestPaymentError> {
        unimplemented!()
    }

    fn get_bucket_tagging(
        &self,
        input: GetBucketTaggingRequest,
    ) -> RusotoFuture<GetBucketTaggingOutput, GetBucketTaggingError> {
        unimplemented!()
    }

    fn get_bucket_versioning(
        &self,
        input: GetBucketVersioningRequest,
    ) -> RusotoFuture<GetBucketVersioningOutput, GetBucketVersioningError> {
        unimplemented!()
    }

    fn get_bucket_website(
        &self,
        input: GetBucketWebsiteRequest,
    ) -> RusotoFuture<GetBucketWebsiteOutput, GetBucketWebsiteError> {
        unimplemented!()
    }

    fn get_object(&self, input: GetObjectRequest) -> RusotoFuture<GetObjectOutput, GetObjectError> {
        unimplemented!()
    }

    fn get_object_acl(
        &self,
        input: GetObjectAclRequest,
    ) -> RusotoFuture<GetObjectAclOutput, GetObjectAclError> {
        unimplemented!()
    }

    fn get_object_legal_hold(
        &self,
        input: GetObjectLegalHoldRequest,
    ) -> RusotoFuture<GetObjectLegalHoldOutput, GetObjectLegalHoldError> {
        unimplemented!()
    }

    fn get_object_lock_configuration(
        &self,
        input: GetObjectLockConfigurationRequest,
    ) -> RusotoFuture<GetObjectLockConfigurationOutput, GetObjectLockConfigurationError> {
        unimplemented!()
    }

    fn get_object_retention(
        &self,
        input: GetObjectRetentionRequest,
    ) -> RusotoFuture<GetObjectRetentionOutput, GetObjectRetentionError> {
        unimplemented!()
    }

    fn get_object_tagging(
        &self,
        input: GetObjectTaggingRequest,
    ) -> RusotoFuture<GetObjectTaggingOutput, GetObjectTaggingError> {
        unimplemented!()
    }

    fn get_object_torrent(
        &self,
        input: GetObjectTorrentRequest,
    ) -> RusotoFuture<GetObjectTorrentOutput, GetObjectTorrentError> {
        unimplemented!()
    }

    fn get_public_access_block(
        &self,
        input: GetPublicAccessBlockRequest,
    ) -> RusotoFuture<GetPublicAccessBlockOutput, GetPublicAccessBlockError> {
        unimplemented!()
    }

    fn head_bucket(&self, input: HeadBucketRequest) -> RusotoFuture<(), HeadBucketError> {
        unimplemented!()
    }

    fn head_object(
        &self,
        input: HeadObjectRequest,
    ) -> RusotoFuture<HeadObjectOutput, HeadObjectError> {
        unimplemented!()
    }

    fn list_bucket_analytics_configurations(
        &self,
        input: ListBucketAnalyticsConfigurationsRequest,
    ) -> RusotoFuture<ListBucketAnalyticsConfigurationsOutput, ListBucketAnalyticsConfigurationsError>
    {
        unimplemented!()
    }

    fn list_bucket_inventory_configurations(
        &self,
        input: ListBucketInventoryConfigurationsRequest,
    ) -> RusotoFuture<ListBucketInventoryConfigurationsOutput, ListBucketInventoryConfigurationsError>
    {
        unimplemented!()
    }

    fn list_bucket_metrics_configurations(
        &self,
        input: ListBucketMetricsConfigurationsRequest,
    ) -> RusotoFuture<ListBucketMetricsConfigurationsOutput, ListBucketMetricsConfigurationsError>
    {
        unimplemented!()
    }

    fn list_buckets(&self) -> RusotoFuture<ListBucketsOutput, ListBucketsError> {
        unimplemented!()
    }

    fn list_multipart_uploads(
        &self,
        input: ListMultipartUploadsRequest,
    ) -> RusotoFuture<ListMultipartUploadsOutput, ListMultipartUploadsError> {
        unimplemented!()
    }

    fn list_object_versions(
        &self,
        input: ListObjectVersionsRequest,
    ) -> RusotoFuture<ListObjectVersionsOutput, ListObjectVersionsError> {
        unimplemented!()
    }

    fn list_objects(
        &self,
        input: ListObjectsRequest,
    ) -> RusotoFuture<ListObjectsOutput, ListObjectsError> {
        unimplemented!()
    }

    fn list_objects_v2(
        &self,
        input: ListObjectsV2Request,
    ) -> RusotoFuture<ListObjectsV2Output, ListObjectsV2Error> {
        unimplemented!()
    }

    fn list_parts(&self, input: ListPartsRequest) -> RusotoFuture<ListPartsOutput, ListPartsError> {
        unimplemented!()
    }

    fn put_bucket_accelerate_configuration(
        &self,
        input: PutBucketAccelerateConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketAccelerateConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_acl(&self, input: PutBucketAclRequest) -> RusotoFuture<(), PutBucketAclError> {
        unimplemented!()
    }

    fn put_bucket_analytics_configuration(
        &self,
        input: PutBucketAnalyticsConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketAnalyticsConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_cors(&self, input: PutBucketCorsRequest) -> RusotoFuture<(), PutBucketCorsError> {
        unimplemented!()
    }

    fn put_bucket_encryption(
        &self,
        input: PutBucketEncryptionRequest,
    ) -> RusotoFuture<(), PutBucketEncryptionError> {
        unimplemented!()
    }

    fn put_bucket_inventory_configuration(
        &self,
        input: PutBucketInventoryConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketInventoryConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_lifecycle(
        &self,
        input: PutBucketLifecycleRequest,
    ) -> RusotoFuture<(), PutBucketLifecycleError> {
        unimplemented!()
    }

    fn put_bucket_lifecycle_configuration(
        &self,
        input: PutBucketLifecycleConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketLifecycleConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_logging(
        &self,
        input: PutBucketLoggingRequest,
    ) -> RusotoFuture<(), PutBucketLoggingError> {
        unimplemented!()
    }

    fn put_bucket_metrics_configuration(
        &self,
        input: PutBucketMetricsConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketMetricsConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_notification(
        &self,
        input: PutBucketNotificationRequest,
    ) -> RusotoFuture<(), PutBucketNotificationError> {
        unimplemented!()
    }

    fn put_bucket_notification_configuration(
        &self,
        input: PutBucketNotificationConfigurationRequest,
    ) -> RusotoFuture<(), PutBucketNotificationConfigurationError> {
        unimplemented!()
    }

    fn put_bucket_policy(
        &self,
        input: PutBucketPolicyRequest,
    ) -> RusotoFuture<(), PutBucketPolicyError> {
        unimplemented!()
    }

    fn put_bucket_replication(
        &self,
        input: PutBucketReplicationRequest,
    ) -> RusotoFuture<(), PutBucketReplicationError> {
        unimplemented!()
    }

    fn put_bucket_request_payment(
        &self,
        input: PutBucketRequestPaymentRequest,
    ) -> RusotoFuture<(), PutBucketRequestPaymentError> {
        unimplemented!()
    }

    fn put_bucket_tagging(
        &self,
        input: PutBucketTaggingRequest,
    ) -> RusotoFuture<(), PutBucketTaggingError> {
        unimplemented!()
    }

    fn put_bucket_versioning(
        &self,
        input: PutBucketVersioningRequest,
    ) -> RusotoFuture<(), PutBucketVersioningError> {
        unimplemented!()
    }

    fn put_bucket_website(
        &self,
        input: PutBucketWebsiteRequest,
    ) -> RusotoFuture<(), PutBucketWebsiteError> {
        unimplemented!()
    }

    fn put_object(&self, input: PutObjectRequest) -> RusotoFuture<PutObjectOutput, PutObjectError> {
        unimplemented!()
    }

    fn put_object_acl(
        &self,
        input: PutObjectAclRequest,
    ) -> RusotoFuture<PutObjectAclOutput, PutObjectAclError> {
        unimplemented!()
    }

    fn put_object_legal_hold(
        &self,
        input: PutObjectLegalHoldRequest,
    ) -> RusotoFuture<PutObjectLegalHoldOutput, PutObjectLegalHoldError> {
        unimplemented!()
    }

    fn put_object_lock_configuration(
        &self,
        input: PutObjectLockConfigurationRequest,
    ) -> RusotoFuture<PutObjectLockConfigurationOutput, PutObjectLockConfigurationError> {
        unimplemented!()
    }

    fn put_object_retention(
        &self,
        input: PutObjectRetentionRequest,
    ) -> RusotoFuture<PutObjectRetentionOutput, PutObjectRetentionError> {
        unimplemented!()
    }

    fn put_object_tagging(
        &self,
        input: PutObjectTaggingRequest,
    ) -> RusotoFuture<PutObjectTaggingOutput, PutObjectTaggingError> {
        unimplemented!()
    }

    fn put_public_access_block(
        &self,
        input: PutPublicAccessBlockRequest,
    ) -> RusotoFuture<(), PutPublicAccessBlockError> {
        unimplemented!()
    }

    fn restore_object(
        &self,
        input: RestoreObjectRequest,
    ) -> RusotoFuture<RestoreObjectOutput, RestoreObjectError> {
        unimplemented!()
    }

    fn select_object_content(
        &self,
        input: SelectObjectContentRequest,
    ) -> RusotoFuture<SelectObjectContentOutput, SelectObjectContentError> {
        unimplemented!()
    }

    fn upload_part(
        &self,
        input: UploadPartRequest,
    ) -> RusotoFuture<UploadPartOutput, UploadPartError> {
        unimplemented!()
    }

    fn upload_part_copy(
        &self,
        input: UploadPartCopyRequest,
    ) -> RusotoFuture<UploadPartCopyOutput, UploadPartCopyError> {
        unimplemented!()
    }
}

/// Only defines `put_object`, failing `max_fails` times before succeeding
#[derive(Clone, Debug)]
pub struct S3MockRetry {
    max_fails: usize,
    fails: Arc<Mutex<usize>>,
}
impl S3MockRetry {
    pub fn new(max_fails: usize) -> S3MockRetry {
        S3MockRetry {
            max_fails,
            fails: Arc::new(Mutex::new(0)),
        }
    }
}
#[trait_impl]
impl S3WithDefaults for S3MockRetry {
    fn put_object(
        &self,
        _input: PutObjectRequest,
    ) -> RusotoFuture<PutObjectOutput, PutObjectError> {
        let curr_fails = *self.fails.lock().unwrap();
        if curr_fails == self.max_fails {
            // succeed
            RusotoFuture::from_future(Ok(PutObjectOutput::default()))
        } else {
            // fail
            *self.fails.lock().unwrap() += 1;
            RusotoFuture::from_future(Err(RusotoError::HttpDispatch(
                request::HttpDispatchError::new("timeout".into()),
            )))
        }
    }
}

// TODO https://github.com/rusoto/rusoto/issues/1546
/// Only defines `put_object`, failing `max_fails` times before succeeding, where 'failing' means
/// taking an eternity
#[derive(Clone, Debug)]
pub struct S3MockTimeout {
    max_fails: usize,
    fails: Arc<Mutex<usize>>,
}
impl S3MockTimeout {
    #[allow(unused)] // waiting for issue..
    pub fn new(max_fails: usize) -> S3MockTimeout {
        S3MockTimeout {
            max_fails,
            fails: Arc::new(Mutex::new(0)),
        }
    }
}
#[trait_impl]
impl S3WithDefaults for S3MockTimeout {
    fn put_object(
        &self,
        _input: PutObjectRequest,
    ) -> RusotoFuture<PutObjectOutput, PutObjectError> {
        let curr_fails = *self.fails.lock().unwrap();
        if curr_fails == self.max_fails {
            // succeed
            RusotoFuture::from_future(Ok(PutObjectOutput::default()))
        } else {
            // fail
            *self.fails.lock().unwrap() += 1;
            RusotoFuture::from_future(
                tokio::timer::Delay::new(Instant::now() + Duration::from_secs(10))
                    .map(|_| PutObjectOutput::default())
                    .map_err(|_| {
                        RusotoError::HttpDispatch(request::HttpDispatchError::new("timeout".into()))
                    }),
            )
        }
    }
}
