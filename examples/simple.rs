use futures::future::ok;
use s3_algo::*;

#[tokio::main]
async fn main() {
    const N_FILES: usize = 10;
    let files =
        (0..N_FILES).map(|i| ObjectSource::data(format!("hey, {}", i), format!("hey{}", i)));
    s3_upload_files(
        testing_s3_client(),
        "test-bucket".into(),
        files,
        UploadConfig::default(),
        |result| println!("File {}/{} successfully uploaded", result.seq + 1, N_FILES),
        Default::default,
    )
    .await
    .unwrap();
}
