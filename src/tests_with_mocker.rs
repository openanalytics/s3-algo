
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
async fn test_s3_single_request() {
    // Test timeout of a single request - that it is not too low

    const ACTUAL_SPEED: f32 = 5_000_000.0; // bytes per sec
    const SIZE: usize = 5_000_000;
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
async fn test_delete_files_parallelization() {
    // List 10 pages of file - each listing takes 100ms,
    // and delete pages in parallel (100ms per page)

    let s3 = S3MockListAndDelete::new(Duration::from_millis(100), Duration::from_millis(100), 10);
    let algo = S3Algo::new(s3);

    let start = Instant::now();
    algo.list_prefix("test-bucket".to_string(), "some/prefix".to_string())
        .delete_all(|_| async {}, |_| async {})
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
