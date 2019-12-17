use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct UploadConfig {
    /// Maximum number of simultaneous upload requests
    pub copy_parallelization: usize,
    /// Timeout is set to a fraction of expected upload time (> 1.0)
    pub timeout_fraction: f64,
    /// Every retry, the timeout is multiplied by backoff (> 1.0)
    pub backoff: f64,
    /// Number of times to retry a single request before giving up
    pub n_retries: usize,
    /// Expected upload speed in MBps (megabytes per second) - used as an initial
    /// estimate.
    pub expected_upload_speed: f64,
    /// To estimate the upload speed incrementally, we use an exponential average:
    /// `new_avg_speed = avg_power * new_speed + (1 - avg_power) * avg_speed`.
    ///
    /// Thus, between 0.0 and 1.0, closer to 1.0 means that newer data points have
    /// more significance.
    pub avg_power: f64,
    /// Only results from uploads larger than `avg_min_bytes` are used to estimate
    /// upload speed.
    /// Uploads with size below this threshold get timeout set to `min_timeout`.
    pub avg_min_bytes: u64,
    /// The minimum timeout (seconds) (always added as an extra term to the
    /// total timeout)
    pub min_timeout: f64,
    /// For testing/demo purposes, the copy job sleeps for this amount of seconds after copying.
    pub extra_copy_time_s: u64,
    /// For testing/demo purposes, the copy job sleeps for this amount of seconds after copying
    /// each file.
    pub extra_copy_file_time_s: u64,
}
impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            copy_parallelization: 20,
            timeout_fraction: 1.5,
            backoff: 1.3,
            n_retries: 8,
            expected_upload_speed: 1.0,
            avg_power: 0.7,
            avg_min_bytes: 1_000_000,
            min_timeout: 0.5,
            extra_copy_time_s: 0,
            extra_copy_file_time_s: 0,
        }
    }
}
