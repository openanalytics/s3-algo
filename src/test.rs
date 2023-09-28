use crate::{mock::*, *};
use rand::Rng;
use rusoto_s3::*;
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
    fn get_timeout(&self, _bytes: usize, _attempts: usize) -> Duration {
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
        .map(|x| x as char)
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
        |_, size| size,
        5,
        Arc::new(Mutex::new(TimeoutState::new(
            AlgorithmConfig::default(),
            SpecificTimings::default_for_bytes(),
        ))),
    ))
}

#[tokio::test]
async fn test_s3_upload_files() {
    const N_FILES: usize = 100;
    let tmp_dir = TempDir::new("s3-testing").unwrap();

    let s3 = testing_sdk_client().await;
    let algo = S3Algo::new(s3.clone());
    let dir_key = upload_test_files(algo.clone(), tmp_dir.path(), N_FILES)
        .await
        .unwrap();

    // Check that all files are there
    for i in 0..N_FILES {
        // let key = format!("{}/img_{}.tif", dir_key, i);
        let key = dir_key.join(format!("img_{}.tif", i));

        let response = s3
            .get_object()
            .bucket("test-bucket".to_string())
            .key(key.to_str().unwrap().to_string())
            .send()
            .await
            .unwrap();

        let mut body = response.body.into_async_read();
        let mut content = Vec::new();
        body.read_to_end(&mut content).await.unwrap();
        let content = std::str::from_utf8(&content).unwrap();
        assert_eq!(content, "file contents");
    }
}

#[tokio::test]
async fn test_s3_timeouts() {
    // TODO finish test
    // Currently just prints things to inspect how timeout behaves

    let bytes: Vec<usize> = vec![500_000, 999_999, 1_000_001, 2_000_000];
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

/// Returns the common prefix of all files in S3
async fn upload_test_files(s3: S3Algo, parent: &Path, n_files: usize) -> Result<PathBuf, Error> {
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
        |client| client.put_object(),
    )
    .await?;
    Ok(dir_key)
}

// TODO uncomment after rewriting move_all function ETC
/*
#[tokio::test]
async fn test_move_files() {
    const N_FILES: usize = 100;
    let s3 = testing_sdk_client().await;
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
    algo.list_prefix("test-bucket".into(), prefix.to_str().map(|x| x.to_owned()))
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
*/

// TODO: uncomment after rewriting copy_all function
/*
#[tokio::test]
async fn test_copy_files() {
    const N_FILES: usize = 100;
    let s3 = testing_s3_client();
    let algo = S3Algo::new(s3.clone());
    let tmp_dir = TempDir::new("s3-testing").unwrap();
    let prefix = upload_test_files(algo.clone(), tmp_dir.path(), N_FILES)
        .await
        .unwrap();

    let n = Arc::new(std::sync::Mutex::new(0_usize));
    let m = n.clone();
    algo.list_prefix("test-bucket".into(), prefix.to_str().unwrap().to_owned())
        .boxed() // hope we can remove boxed() soon (it's for reducing type size)
        .copy_all(
            Some("test-bucket2".into()),
            move |key| {
                *m.lock().unwrap() += 1;
                format!("test_copy_files/{}", key)
            },
            Default::default,
        )
        .boxed()
        .await
        .unwrap();
    assert_eq!(*n.lock().unwrap(), N_FILES);

    // Check that all objects are present in both buckets
    for i in 0..N_FILES {
        let key = format!("test_copy_files/{}/img_{}.tif", prefix.display(), i);
        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket2".to_string(),
            key,
            ..Default::default()
        });
        let _ = response.await.unwrap();

        let key = prefix.join(format!("img_{}.tif", i));
        let response = s3.get_object(GetObjectRequest {
            bucket: "test-bucket".to_string(),
            key: key.to_str().unwrap().to_string(),
            ..Default::default()
        });
        let _ = response.await.unwrap();
    }
}
*/
