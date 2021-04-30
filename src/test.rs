use crate::{mock::*, *};
use rand::Rng;
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use tempdir::TempDir;
use timeout::TimeoutState;
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;

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
    // This is only to test that it compiles
    fn verify<F, T>(_: F)
    where
        F: Future<Output = T> + Send + 'static,
    {
    }

    verify(s3_request(
        || async move { Ok((async move { Ok(()) }, 0)) },
        5,
        Arc::new(Mutex::new(TimeoutState::new(
            AlgorithmConfig::default(),
            SpecificTimings::default_for_bytes(),
        ))),
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

    let s3 = S3Algo::new(S3MockRetry::new(2));

    let counter = Arc::new(Mutex::new(0));
    s3.upload_files(
        "any-bucket".into(),
        files_recursive(folder, PathBuf::new()),
        move |res| {
            let counter = counter.clone();
            async move {
                let mut counter = counter.lock().await;
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

    let s3 = S3Algo::new(S3MockRetry::new(ATTEMPTS - 1));

    s3.upload_files(
        "any_bucket".into(),
        files_recursive(path, PathBuf::new()),
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

    let s3 = testing_s3_client();
    let algo = S3Algo::new(s3.clone());
    let dir_key = upload_test_files(algo.clone(), tmp_dir.path(), N_FILES)
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

    let cli = S3MockBps::new(ACTUAL_SPEED);
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
        0.5,
    )
    .await;

    let _ = result.unwrap();
}

#[tokio::test]
async fn test_s3_timeouts() {
    // TODO finish test
    // Currently just prints things to inspect how timeout behaves

    let bytes: Vec<u64> = vec![500_000, 999_999, 1_000_001, 2_000_000];
    // Test that timeout on successive errors follows a desired curve

    // These are all parameters related to timeout, shown explicitly
    let cfg = Config {
        algorithm: AlgorithmConfig {
            backoff: 1.5,
            base_timeout: 0.5,
            timeout_fraction: 1.5,
            avg_power: 0.7,
            ..Default::default()
        },
        ..Default::default()
    };

    for bytes in bytes {
        println!("# Bytes = {}", bytes);
        let timeout = TimeoutState::new(cfg.algorithm.clone(), cfg.put_requests.clone());

        let timeouts = (1..=10)
            .map(|retries| timeout.get_timeout(bytes, retries))
            .collect::<Vec<_>>();
        println!("{:?}", timeouts);
    }
}

#[tokio::test]
async fn test_delete_files_parallelization() {
    // List 10 pages of file - each listing takes 100ms,
    // and delete pages in parallel (100ms per page)

    let s3 = S3MockListAndDelete::new(Duration::from_millis(100), Duration::from_millis(100), 10);
    let algo = S3Algo::new(s3);

    let start = Instant::now();
    algo.list_prefix("test-bucket".to_string(), "some/prefix".to_string())
        .delete_all(|_| async {})
        .await
        .unwrap();
    let duration = Instant::now() - start;
    println!("Duration: {}", duration.as_millis());

    // In the case that listing and deleting is not parallelized, we will have at least 20 requests
    // each of 100ms:
    assert!(duration.as_millis() < 2000);
    // TODO: I think this now passes because delete operations are concurrent, but I'm unsure if
    // listing and deletion are concurrent wrt each other. TODO: take a look at `parallel_stream`
    //
}

/// Returns the common prefix of all files in S3
async fn upload_test_files<S: S3 + Clone + Send + Sync + Unpin + 'static>(
    s3: S3Algo<S>,
    parent: &Path,
    n_files: usize,
) -> Result<PathBuf, Error> {
    let dir_key = Path::new(&rand_string(4))
        .join(rand_string(4))
        .join(rand_string(4));
    let dir = parent.join(&dir_key);
    std::fs::create_dir_all(&dir).unwrap();
    for i in 0..n_files {
        std::fs::write(dir.join(format!("img_{}.tif", i)), "file contents").unwrap();
    }

    println!("Upload {} to {:?} ", dir.display(), dir_key);
    s3.upload_files(
        "test-bucket".into(),
        files_recursive(dir.clone(), dir.strip_prefix(parent).unwrap().to_owned()),
        |_| async move {},
        PutObjectRequest::default,
    )
    .await?;
    Ok(dir_key)
}
#[tokio::test]
async fn test_move_files() {
    const N_FILES: usize = 100;
    let s3 = testing_s3_client();
    let algo = S3Algo::new(s3.clone());
    let tmp_dir = TempDir::new("s3-testing").unwrap();
    let prefix = upload_test_files(algo.clone(), tmp_dir.path(), N_FILES)
        .await
        .unwrap();
    let new_prefix = PathBuf::from("haha/lala");
    println!(
        "Move prefix {} to {}",
        prefix.display(),
        new_prefix.display()
    );

    // TODO try also the following more manual way of doing the same
    /*
    algo.list_prefix("test-bucket".into(), format!("{}", prefix.display()))
        .boxed() // hope we can remove boxed() soon (it's for reducing type size)
        .move_all(
            move |key| {
                let key = PathBuf::from(key);
                let name = key.file_name().unwrap();
                format!("{}/{}", new_prefix2.display(), name.to_str().unwrap())
            },
            None,
        )
        .await
        .unwrap();
    */
    algo.list_prefix("test-bucket".into(), prefix.to_str().unwrap().to_owned())
        .boxed() // hope we can remove boxed() soon (it's for reducing type size)
        .move_to_prefix(
            None,
            new_prefix.to_str().unwrap().to_owned(),
            Default::default,
        )
        .boxed()
        .await
        .unwrap();

    // Check that all files are under `new_prefix` and not under `prefix`
    for i in 0..N_FILES {
        let key = new_prefix.join(format!("img_{}.tif", i));
        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket".to_string(),
            key: key.to_str().unwrap().to_string(),
            ..Default::default()
        });
        let _ = response.await.unwrap();

        let key = prefix.join(format!("img_{}.tif", i));
        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket".to_string(),
            key: key.to_str().unwrap().to_string(),
            ..Default::default()
        });
        let _ = response.await.unwrap_err();
    }
}
