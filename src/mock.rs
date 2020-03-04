use crate::*;
use multi_default_trait_impl::{default_trait_impl, trait_impl};
use rusoto_core::*;
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::delay_for;
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
/// Simulates that an operation actually takes some time to complete (only supports put_object)
/// based on the content_length field.
#[derive(Clone, Debug)]
pub struct S3MockTimeout {
    bps: f32,
}
impl S3MockTimeout {
    /// `bps`: bytes per sec
    pub fn new(bps: f32) -> S3MockTimeout {
        S3MockTimeout { bps }
    }
}
#[trait_impl]
impl S3WithDefaults for S3MockTimeout {
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
        // succeed
        let seconds = input.content_length.unwrap_or(0) as f32 / self.bps;
        Box::pin(async move {
            delay_for(Duration::from_millis((seconds * 1000.0) as u64)).await;
            Ok(PutObjectOutput::default())
        })
    }
}
