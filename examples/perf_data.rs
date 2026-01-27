use clap::{Arg, Command};
use s3_algo::*;
use std::io::Write;
use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    let matches = Command::new("perf_data")
        .about("Upload a directory to S3 on localhost.")
        .arg(
            Arg::new("source")
                .help("Path to a folder to upload to S3")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("dest_bucket")
                .help("Destination bucket")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("dest_prefix")
                .help("Destination prefix")
                .required(true)
                .index(3),
        )
        .arg(
            Arg::new("parallelization")
                .short('n')
                .long("parallelization")
                .value_name("N")
                .help("Maximum number of simultaneous upload requests"),
        )
        .get_matches();

    let path = matches.get_one::<String>("source").unwrap();
    let bucket = matches.get_one::<String>("dest_bucket").unwrap();
    let prefix = matches.get_one::<String>("dest_prefix").unwrap();
    let parallelization = matches
        .get_one::<String>("parallelization")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);

    benchmark_s3_upload(
        Path::new(path).to_path_buf(),
        bucket.to_owned(),
        prefix.to_owned(),
        parallelization,
    )
    .await;
    println!("Done");
}

async fn benchmark_s3_upload(
    dir_path: PathBuf,
    bucket: String,
    prefix: String,
    copy_parallelization: usize,
) {
    let cfg = Config {
        copy_parallelization,
        ..Default::default()
    };
    let s3 = testing_sdk_client().await;
    let algo = S3Algo::with_config(s3, cfg);

    upload_perf_log_init(&mut std::io::stdout());
    let progress = |res| async move { upload_perf_log_update(&mut std::io::stdout(), res) };

    algo.upload_files(
        bucket,
        files_recursive(dir_path, PathBuf::from(&prefix)),
        progress,
        |client| client.put_object(),
    )
    .await
    .unwrap();
}

// Helpers for writing data
macro_rules! write_cell {
    ($out:expr, $x:expr) => {
        let _ = write!($out, "{0: >18}", format!("{:.5}", $x));
    };
}
pub fn upload_perf_log_init<W: Write>(out: &mut W) {
    let _ = writeln!(
        out,
        "{0: >w$}{1: >w$}{2: >w$}{3: >w$}{4: >w$}{5: >w$}",
        "attempts",
        "bytes",
        "success_ms",
        "total_ms",
        "MBps",
        "MBps est",
        w = 18
    );
}
pub fn upload_perf_log_update<W: Write>(out: &mut W, res: RequestReport) {
    // TODO: Write performance data to file with tokio
    let megabytes = res.size as f64 / 1_000_000.0;
    let speed = megabytes / res.success_time.as_secs_f64();
    write_cell!(out, res.attempts);
    write_cell!(out, res.size);
    write_cell!(out, res.success_time.as_millis());
    write_cell!(out, res.total_time.as_millis());
    write_cell!(out, speed);
    write_cell!(out, res.est);
    let _ = writeln!(out);
}
