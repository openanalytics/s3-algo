# `s3-algo`

High-performance algorithms for batch operations in Amazon S3, on top of [rusoto](https://github.com/rusoto/rusoto)

https://docs.aws.amazon.com/AmazonS3/latest/dev/optimizing-performance-guidelines.html

Per now, uploading multiple files has been the main focus.
Deletion of prefix is also implemented.
Listing of files is planned.

NOTE: futures from `futures v0.1` are used internally and thus for example `tokio-compat` is
required to run the futures, until `rusoto` updates to use `tokio 0.2`.

NOTE: also due to `rusoto` currently using `bytes 0.4`, a quick compatibility function between
`bytes 0.4` and `bytes 0.5` was made that is not zero-cost (clones the bytes), which will affect
performance of the upload functionality, until `rusoto` updates to `tokio 0.2` and `bytes 0.5`.

# Upload
## Features of the `s3_upload_files` function
* As generic as possible, to support many use cases.
* It is possible to collect detailed data from the upload through a closure - one can choose to use this data to analyze performance, or for example to implement a live progress percentage report.
* Backoff mechanism
* Fast. Several mechanisms are in place, such as [aggressive timeouts](https://docs.aws.amazon.com/AmazonS3/latest/dev/optimizing-performance-guidelines.html), parallelization and streaming files from file system while uploading.

## Algorithm details
The documentation for `UploadConfig` may help illuminate the components of the algorithm.
The currnetly most important aspect of the algorithm revolves around deciding timeout values. That is, how long to wait for a request before trying again.
It is important for performance that the timeout is tight enough.
The main mechanism to this end is the estimation of the upload bandwidth through a running exponential average of the upload speed (on success) of individual files.
Additionally, on each successive retry, the timeout increases by some factor (back-off).

## Yet to consider
* Is the algorithm considerate with respect to other processes that want to use the same network? For example in the case of congestion. It does implement increasing back-off intervals after failed requests, but the real effect on a shared network should be tested.


## Examples
### `perf_data`
Command-line interface for uploading any directory to any bucket and prefix in a locally running S3 service (such as `minio`).
Example:
```
cargo run --example perf_data  -- -n 3 ./src test-bucket lala
```

Prints:
```
          attempts             bytes        success_ms          total_ms              MBps          MBps est
                 1              1990                32                32           0.06042           1.00000
                 1             24943                33                33           0.74043           1.00000
                 1              2383                29                29           0.08211           1.00000
                 1               417                13                13           0.03080           1.00000
                 1              8562                16                16           0.51480           1.00000
```
`total_ms` is the total time including all retries, and `success_ms` is the time of only the last attempt.
The distinction between these two is useful in real cases where `attempts` is not always `1`.

You can then verify that the upload happened by entering the container. Something like:

```
$ docker exec -it $(docker ps --filter "ancestor=minio" --format "{{.Names}}") bash
[user@144aff4dae5b ~]$ ls s3/
test-bucket/ 
[user@144aff4dae5b ~]$ ls s3/test-bucket/
lala
```

## Running tests and examples
Both tests and examples require that an S3 service such as `minio` is running locally at port 9000.
Tests assume that a credentials profile exists - for example in `~/.aws/credentials`:

```
[testing]
aws_access_key_id = 123456789
aws_secret_access_key = 123456789
```

After 
