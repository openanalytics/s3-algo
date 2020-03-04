use crate::{mock::*, timeout::Timeout, *};
use rand::Rng;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tempdir::TempDir;
use timeout::TimeoutState;
use tokio::io::AsyncReadExt;

/*
/// Timeout implementation used for testing
struct TimeoutState;
impl Timeout for TimeoutState {
    fn get_timeout(&self, _bytes: u64, _attempts: usize) -> Duration {
        Duration::from_secs(4)
    }
    fn update(&mut self, _: &RequestReport) {}
    fn get_estimate(&self) -> f64 {
        0.0
    }
}
*/

pub(crate) fn rand_string(n: usize) -> String {
    rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(n)
        .collect::<String>()
}

#[test]
fn everything_is_sync_and_static() {
    fn verify<F, T>(_: F)
    where
        F: Future<Output = T> + Send + 'static,
    {
    }

    verify(s3_request(
        || async move { Ok((async move { Ok(()) }, 0)) },
        5,
        Arc::new(Mutex::new(TimeoutState::new(UploadConfig::default()))),
    ))
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

    let counter = Arc::new(Mutex::new(0));
    s3_upload_files(
        cli,
        "any-bucket".into(),
        files_recursive(folder, PathBuf::new()),
        UploadConfig::default(),
        move |res| {
            let counter = counter.clone();
            async move {
                let mut counter = counter.lock().unwrap();
                assert_eq!(*counter, res.seq);
                *counter += 1;
            }
        },
        PutObjectRequest::default,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn s3_upload_file_attempts_count() {
    const ATTEMPTS: usize = 4;
    let tmp_dir = TempDir::new("s3-testing").unwrap();

    let path = tmp_dir.path().join("file.txt");
    std::fs::write(&path, "hello".as_bytes()).unwrap();

    let cli = S3MockRetry::new(ATTEMPTS - 1);

    s3_upload_files(
        cli,
        "any_bucket".into(),
        files_recursive(path, PathBuf::new()),
        UploadConfig::default(),
        |result| async move { assert_eq!(result.attempts, ATTEMPTS) },
        PutObjectRequest::default,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn test_s3_upload_files() {
    const N_FILES: usize = 100;
    let tmp_dir = TempDir::new("s3-testing").unwrap();

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
        files_recursive(
            dir.clone(),
            dir.strip_prefix(tmp_dir.path()).unwrap().to_owned(),
        ),
        UploadConfig::default(),
        |_| async move {},
        PutObjectRequest::default,
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

#[tokio::test]
async fn test_s3_single_request() {
    // Test timeout of a single request - that it is not too low

    const ACTUAL_SPEED: f32 = 5_000_000.0; // bytes per sec
    const SIZE: u64 = 5_000_000;
    let dir = tempdir::TempDir::new("testing").unwrap();
    let path = dir.path().join("file.txt");
    std::fs::write(&path, "file contents").unwrap();

    let cli = S3MockTimeout::new(ACTUAL_SPEED);
    let result = s3_single_request(
        move || {
            let cli = cli.clone();
            async move {
                cli.put_object(PutObjectRequest {
                    bucket: "testing".into(),
                    key: "hey".into(),
                    body: Some(vec![].into()),
                    content_length: Some(SIZE as i64),
                    ..Default::default()
                })
                .map_ok(drop)
                .await
                .context(err::PutObject {
                    key: "hey".to_string(),
                })
            }
        },
        Some(SIZE),
    )
    .await;

    let _ = result.unwrap();
}
