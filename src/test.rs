use crate::testing_s3_client;
use crate::{timeout::Timeout, *};
use futures::future::ok;
use multi_default_trait_impl::{default_trait_impl, trait_impl};
use rand::Rng;
use rusoto_core::*;
use std::{
    path::Path,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tempdir::TempDir;
use tokio::{io::AsyncReadExt, time::delay_for};

/// Timeout implementation used for testing
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

fn rand_string(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(n)
        .collect::<String>()
}

#[tokio::test]
async fn s3_upload_files_seq_count() {
    const N_FILES: usize = 100;
    let tmp_dir = TempDir::new("s3-testing").unwrap();
    let folder = tmp_dir.path().join("folder");
    std::fs::create_dir_all(&folder).unwrap();

    for i in 0..N_FILES {
        let path = folder.join(format!("file_{}", i));
        std::fs::write(&path, "hello".as_bytes()).unwrap();
    }

    let cli = S3MockRetry::new(2);

    let counter = Mutex::new(0);
    s3_upload_files(
        cli,
        "any-bucket".into(),
        files_recursive(folder, PathBuf::new()),
        UploadConfig::default(),
        move |res| {
            let mut counter = counter.lock().unwrap();
            assert_eq!(*counter, res.seq);
            *counter += 1;
            ok(())
        },
        PutObjectRequest::default,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn s3_upload_file_attempts_count() {
    let timeout = Arc::new(Mutex::new(TimeoutState));

    const ATTEMPTS: usize = 4;
    let tmp_dir = TempDir::new("s3-testing").unwrap();

    let path = tmp_dir.path().join("file.txt");
    std::fs::write(&path, "hello".as_bytes()).unwrap();

    let cli = S3MockRetry::new(ATTEMPTS - 1);

    let result = s3_upload_files(
        cli,
        "any_bucket".into(),
        files_recursive(path, PathBuf::new()),
        UploadConfig::default(),
        |result| async move {
            assert_eq!(result.attempts, ATTEMPTS);
            Ok(())
        },
        PutObjectRequest::default
        )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_s3_upload_files() {
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

    let s3 = testing_s3_client();
    println!("Upload {} to {:?} ", dir.display(), dir_key);
    s3_upload_files(
        s3.clone(),
        "test-bucket".into(),
        files_recursive(dir, dir.strip_prefix(tmp_dir).unwrap().to_owned()),
        UploadConfig::default(),
        |result| ok(()),
        PutObjectRequest::default
    )
    .await
    .unwrap();


    // Check that all files are there
    for i in 0..N_FILES {
        // let key = format!("{}/img_{}.tif", dir_key, i);
        let key = dir_key.join(format!("img_{}.tif", i));

        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket".to_string(),
            key: key.to_str().unwrap().to_string(),
            ..Default::default()
        });
        let response = response.await.unwrap();

        let mut body = response.body.unwrap().into_async_read();
        let mut content = Vec::new();
        body.read_to_end(&mut content).await.unwrap();
        let content = std::str::from_utf8(&content).unwrap();
        assert_eq!(content, "file contents");
    }
}

// TODO:
// fn test_s3_delete_files
// - and make it delete enough files to trigger paging

#[tokio::test]
async fn test_s3_timeout() {
    // TODO (not sure yet exactly what it's supposed to test)
    // it simulates some timeout anyway
    let dir = tempdir::TempDir::new("testing").unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(&path, "file contents").unwrap();

    let timeout = Arc::new(Mutex::new(TimeoutState));
    let cli = S3MockTimeout::new(1);
    let result = s3_request(
        |attempts| {
            stream_file_to_s3(
                cli.clone(),
                path.clone(),
                "testing".into(),
                "hey".into(),
                timeout.clone(),
                attempts,
                PutObjectRequest::default,
            )
        },
        10,
    )
    .await;

    println!("{:?}", result.unwrap());
}

// Just to be able to easily create mock objects, we create a temporary trait with defaults that
// panic
#[default_trait_impl]
impl S3 for S3WithDefaults {
    fn abort_multipart_upload<'life0, 'async_trait>(
        &'life0 self,
        input: AbortMultipartUploadRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        AbortMultipartUploadOutput,
                        RusotoError<AbortMultipartUploadError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn complete_multipart_upload<'life0, 'async_trait>(
        &'life0 self,
        input: CompleteMultipartUploadRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        CompleteMultipartUploadOutput,
                        RusotoError<CompleteMultipartUploadError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn copy_object<'life0, 'async_trait>(
        &'life0 self,
        input: CopyObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CopyObjectOutput, RusotoError<CopyObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn create_bucket<'life0, 'async_trait>(
        &'life0 self,
        input: CreateBucketRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CreateBucketOutput, RusotoError<CreateBucketError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn create_multipart_upload<'life0, 'async_trait>(
        &'life0 self,
        input: CreateMultipartUploadRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        CreateMultipartUploadOutput,
                        RusotoError<CreateMultipartUploadError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<(), RusotoError<DeleteBucketError>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_analytics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketAnalyticsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketAnalyticsConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_cors<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketCorsRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketCorsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_encryption<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketEncryptionRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketEncryptionError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_inventory_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketInventoryConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketInventoryConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_lifecycle<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketLifecycleRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketLifecycleError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_metrics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketMetricsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketMetricsConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_policy<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketPolicyRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketPolicyError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_replication<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketReplicationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketReplicationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketTaggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_bucket_website<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteBucketWebsiteRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeleteBucketWebsiteError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_object<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<DeleteObjectOutput, RusotoError<DeleteObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_object_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteObjectTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        DeleteObjectTaggingOutput,
                        RusotoError<DeleteObjectTaggingError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_objects<'life0, 'async_trait>(
        &'life0 self,
        input: DeleteObjectsRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<DeleteObjectsOutput, RusotoError<DeleteObjectsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn delete_public_access_block<'life0, 'async_trait>(
        &'life0 self,
        input: DeletePublicAccessBlockRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<DeletePublicAccessBlockError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_accelerate_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketAccelerateConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketAccelerateConfigurationOutput,
                        RusotoError<GetBucketAccelerateConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_acl<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketAclRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketAclOutput, RusotoError<GetBucketAclError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_analytics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketAnalyticsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketAnalyticsConfigurationOutput,
                        RusotoError<GetBucketAnalyticsConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_cors<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketCorsRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketCorsOutput, RusotoError<GetBucketCorsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_encryption<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketEncryptionRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketEncryptionOutput,
                        RusotoError<GetBucketEncryptionError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_inventory_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketInventoryConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketInventoryConfigurationOutput,
                        RusotoError<GetBucketInventoryConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_lifecycle<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketLifecycleRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<GetBucketLifecycleOutput, RusotoError<GetBucketLifecycleError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_lifecycle_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketLifecycleConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketLifecycleConfigurationOutput,
                        RusotoError<GetBucketLifecycleConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_location<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketLocationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<GetBucketLocationOutput, RusotoError<GetBucketLocationError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_logging<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketLoggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketLoggingOutput, RusotoError<GetBucketLoggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_metrics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketMetricsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketMetricsConfigurationOutput,
                        RusotoError<GetBucketMetricsConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_notification<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketNotificationConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        NotificationConfigurationDeprecated,
                        RusotoError<GetBucketNotificationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_notification_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketNotificationConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        NotificationConfiguration,
                        RusotoError<GetBucketNotificationConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_policy<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketPolicyRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketPolicyOutput, RusotoError<GetBucketPolicyError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_policy_status<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketPolicyStatusRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketPolicyStatusOutput,
                        RusotoError<GetBucketPolicyStatusError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_replication<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketReplicationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketReplicationOutput,
                        RusotoError<GetBucketReplicationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_request_payment<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketRequestPaymentRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketRequestPaymentOutput,
                        RusotoError<GetBucketRequestPaymentError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketTaggingOutput, RusotoError<GetBucketTaggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_versioning<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketVersioningRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetBucketVersioningOutput,
                        RusotoError<GetBucketVersioningError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_bucket_website<'life0, 'async_trait>(
        &'life0 self,
        input: GetBucketWebsiteRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetBucketWebsiteOutput, RusotoError<GetBucketWebsiteError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetObjectOutput, RusotoError<GetObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_acl<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectAclRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetObjectAclOutput, RusotoError<GetObjectAclError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_legal_hold<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectLegalHoldRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<GetObjectLegalHoldOutput, RusotoError<GetObjectLegalHoldError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_lock_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectLockConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetObjectLockConfigurationOutput,
                        RusotoError<GetObjectLockConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_retention<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectRetentionRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<GetObjectRetentionOutput, RusotoError<GetObjectRetentionError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetObjectTaggingOutput, RusotoError<GetObjectTaggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_object_torrent<'life0, 'async_trait>(
        &'life0 self,
        input: GetObjectTorrentRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetObjectTorrentOutput, RusotoError<GetObjectTorrentError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn get_public_access_block<'life0, 'async_trait>(
        &'life0 self,
        input: GetPublicAccessBlockRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        GetPublicAccessBlockOutput,
                        RusotoError<GetPublicAccessBlockError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn head_bucket<'life0, 'async_trait>(
        &'life0 self,
        input: HeadBucketRequest,
    ) -> Pin<Box<dyn Future<Output = Result<(), RusotoError<HeadBucketError>>> + Send + 'async_trait>>
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn head_object<'life0, 'async_trait>(
        &'life0 self,
        input: HeadObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<HeadObjectOutput, RusotoError<HeadObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_bucket_analytics_configurations<'life0, 'async_trait>(
        &'life0 self,
        input: ListBucketAnalyticsConfigurationsRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        ListBucketAnalyticsConfigurationsOutput,
                        RusotoError<ListBucketAnalyticsConfigurationsError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_bucket_inventory_configurations<'life0, 'async_trait>(
        &'life0 self,
        input: ListBucketInventoryConfigurationsRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        ListBucketInventoryConfigurationsOutput,
                        RusotoError<ListBucketInventoryConfigurationsError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_bucket_metrics_configurations<'life0, 'async_trait>(
        &'life0 self,
        input: ListBucketMetricsConfigurationsRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        ListBucketMetricsConfigurationsOutput,
                        RusotoError<ListBucketMetricsConfigurationsError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_buckets<'life0, 'async_trait>(
        &'life0 self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ListBucketsOutput, RusotoError<ListBucketsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_multipart_uploads<'life0, 'async_trait>(
        &'life0 self,
        input: ListMultipartUploadsRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        ListMultipartUploadsOutput,
                        RusotoError<ListMultipartUploadsError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_object_versions<'life0, 'async_trait>(
        &'life0 self,
        input: ListObjectVersionsRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<ListObjectVersionsOutput, RusotoError<ListObjectVersionsError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_objects<'life0, 'async_trait>(
        &'life0 self,
        input: ListObjectsRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ListObjectsOutput, RusotoError<ListObjectsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_objects_v2<'life0, 'async_trait>(
        &'life0 self,
        input: ListObjectsV2Request,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ListObjectsV2Output, RusotoError<ListObjectsV2Error>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn list_parts<'life0, 'async_trait>(
        &'life0 self,
        input: ListPartsRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ListPartsOutput, RusotoError<ListPartsError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_accelerate_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketAccelerateConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketAccelerateConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_acl<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketAclRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<(), RusotoError<PutBucketAclError>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_analytics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketAnalyticsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketAnalyticsConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_cors<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketCorsRequest,
    ) -> Pin<
        Box<dyn Future<Output = Result<(), RusotoError<PutBucketCorsError>>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_encryption<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketEncryptionRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketEncryptionError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_inventory_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketInventoryConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketInventoryConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_lifecycle<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketLifecycleRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketLifecycleError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_lifecycle_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketLifecycleConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketLifecycleConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_logging<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketLoggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketLoggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_metrics_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketMetricsConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketMetricsConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_notification<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketNotificationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketNotificationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_notification_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketNotificationConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketNotificationConfigurationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_policy<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketPolicyRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketPolicyError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_replication<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketReplicationRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketReplicationError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_request_payment<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketRequestPaymentRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketRequestPaymentError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketTaggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_versioning<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketVersioningRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketVersioningError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_bucket_website<'life0, 'async_trait>(
        &'life0 self,
        input: PutBucketWebsiteRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutBucketWebsiteError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PutObjectOutput, RusotoError<PutObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object_acl<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectAclRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PutObjectAclOutput, RusotoError<PutObjectAclError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object_legal_hold<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectLegalHoldRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<PutObjectLegalHoldOutput, RusotoError<PutObjectLegalHoldError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object_lock_configuration<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectLockConfigurationRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        PutObjectLockConfigurationOutput,
                        RusotoError<PutObjectLockConfigurationError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object_retention<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectRetentionRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<PutObjectRetentionOutput, RusotoError<PutObjectRetentionError>>,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_object_tagging<'life0, 'async_trait>(
        &'life0 self,
        input: PutObjectTaggingRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PutObjectTaggingOutput, RusotoError<PutObjectTaggingError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn put_public_access_block<'life0, 'async_trait>(
        &'life0 self,
        input: PutPublicAccessBlockRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), RusotoError<PutPublicAccessBlockError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn restore_object<'life0, 'async_trait>(
        &'life0 self,
        input: RestoreObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<RestoreObjectOutput, RusotoError<RestoreObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn select_object_content<'life0, 'async_trait>(
        &'life0 self,
        input: SelectObjectContentRequest,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        SelectObjectContentOutput,
                        RusotoError<SelectObjectContentError>,
                    >,
                > + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn upload_part<'life0, 'async_trait>(
        &'life0 self,
        input: UploadPartRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<UploadPartOutput, RusotoError<UploadPartError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        unimplemented!()
    }
    fn upload_part_copy<'life0, 'async_trait>(
        &'life0 self,
        input: UploadPartCopyRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<UploadPartCopyOutput, RusotoError<UploadPartCopyError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
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
    fn put_object<'life0, 'async_trait>(
        &'life0 self,
        _input: PutObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PutObjectOutput, RusotoError<PutObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let curr_fails = *self.fails.lock().unwrap();
        if curr_fails == self.max_fails {
            // succeed
            Box::pin(async move { Ok(PutObjectOutput::default()) })
        } else {
            // fail
            *self.fails.lock().unwrap() += 1;
            Box::pin(async move {
                Err(RusotoError::HttpDispatch(request::HttpDispatchError::new(
                    "timeout".into(),
                )))
            })
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
    fn put_object<'life0, 'async_trait>(
        &'life0 self,
        _input: PutObjectRequest,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<PutObjectOutput, RusotoError<PutObjectError>>>
                + Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let curr_fails = *self.fails.lock().unwrap();
        println!("Curr fails: {}", curr_fails);
        if curr_fails == self.max_fails {
            // succeed
            Box::pin(async move { Ok(PutObjectOutput::default()) })
        } else {
            // fail
            *self.fails.lock().unwrap() += 1;
            Box::pin(async move {
                delay_for(Duration::from_secs(10)).await;
                println!("Delay for 10 sec?");
                Err(RusotoError::HttpDispatch(request::HttpDispatchError::new(
                    "timeout".into(),
                )))
            })
        }
    }
}
