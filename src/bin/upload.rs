use rusoto_core::{credential::ProfileProvider, region::Region, HttpClient};
use rusoto_s3::*;
use s3_algo::*;

#[tokio::main]
async fn main() {
    /*
    // TODO: next clap and stuff
    s3_upload_files(
        s3_client(&cfg),
        s3_bucket,
        objects.into_iter(),
        cfg.upload.clone(),
        |_| (),
        s3_put_request_factory(cfg.dry_run),
    )
    .context(err::S3)
    */
}

pub fn s3_client() -> S3Client {
    let client = HttpClient::new().unwrap();
    let region = Region::Custom {
        name: "minio".to_owned(),
        endpoint: "http://localhost:9000".to_string(),
    };

    let mut profile = ProfileProvider::new().unwrap();
    profile.set_profile("testing");

    rusoto_s3::S3Client::new_with(client, profile, region)
}
