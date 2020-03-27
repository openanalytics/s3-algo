use s3_algo::*;

#[tokio::main]
async fn main() {
    const N_FILES: usize = 10;
    let files =
        (0..N_FILES).map(|i| ObjectSource::data(format!("hey, {}", i), format!("hey{}", i)));
    let s3 = S3Algo::new(testing_s3_client());
    s3.upload_files(
        "test-bucket".into(),
        files,
        |result| async move { println!("File {}/{} successfully uploaded", result.seq + 1, N_FILES)},
        Default::default,
    )
    .await
    .unwrap();
}
