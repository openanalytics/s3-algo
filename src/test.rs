use crate::{mock::*, timeout::Timeout, *};
use futures::future::{ok, ready};
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

pub(crate) fn rand_string(n: usize) -> String {
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
        |result| async move {
            assert_eq!(result.attempts, ATTEMPTS);
            Ok(())
        },
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
        |_| ok(()),
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

// - and make it delete enough files to trigger paging

/*
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
*/
