//! The `Timeout` trait defines the how the timeout value of a multi-file upload evolves based on
//! past file upload results. A default implementation `TimeoutState` is provided.
use crate::{RequestReport, RequestConfig};
use std::time::Duration;
pub trait Timeout: Send + 'static {
    fn get_timeout(&self, bytes: u64, retries: usize) -> Duration;
    fn update(&mut self, _: &RequestReport);
    /// get estimated upload speed
    fn get_estimate(&self) -> f64;
}
/// State for timeouts in s3_upload_dir_async
pub struct TimeoutState {
    /// current timeout estimate in MBps
    upload_speed_estimate: f64,
    cfg: RequestConfig,
}
impl TimeoutState {
    pub fn new(cfg: RequestConfig) -> TimeoutState {
        TimeoutState {
            upload_speed_estimate: cfg.expected_upload_speed,
            cfg,
        }
    }
}
impl Timeout for TimeoutState {
    fn get_estimate(&self) -> f64 {
        self.upload_speed_estimate
    }
    // TODO/NOTE: there are some commented-out prints. It would probably be convenient to log them
    // to a file (one file each copy job)
    fn get_timeout(&self, bytes: u64, retries: usize) -> Duration {
        let backoff = self.cfg.backoff.powi(retries as i32);
        if bytes > self.cfg.avg_min_bytes {
            let megabytes = bytes as f64 / 1_000_000.0;
            let time_estimate = megabytes / self.upload_speed_estimate * backoff;
            Duration::from_secs_f64(
                self.cfg.min_timeout * backoff + self.cfg.timeout_fraction * time_estimate,
            )
        } else {
            Duration::from_secs_f64(self.cfg.min_timeout * backoff)
        }
    }
    fn update(&mut self, result: &RequestReport) {
        if result.bytes > self.cfg.avg_min_bytes {
            let megabytes = result.bytes as f64 / 1_000_000.0;
            let speed = megabytes / result.success_time.as_secs_f64();
            // println!("File big enough - uploaded with speed {} MBps (MB: {})", speed, megabytes);
            self.upload_speed_estimate = self.cfg.avg_power * self.upload_speed_estimate
                + (1.0 - self.cfg.avg_power) * speed;
        } else {
            // let megabytes = result.bytes as f64 / 1_000_000.0;
            // let speed = megabytes / result.success_time.as_secs_f64();
            // println!("File TOO SMALL - uploaded with speed {} MBps (MB: {})", speed, megabytes);
        }
    }
}
