//! The `Timeout` trait defines the how the timeout value of a multi-file upload evolves based on
//! past file upload results. A default implementation `TimeoutState` is provided.
use crate::config::*;
use crate::RequestReport;
use std::time::Duration;
pub trait Timeout: Send + 'static {
    /// Size is in either bytes or objects, depending on the type of requests.
    fn get_timeout(&self, size: usize, retries: usize) -> Duration;
    /// Update the internal estimate of the extra timeout per unit of size
    fn update(&mut self, _: &RequestReport);
    /// get estimated upload speed
    fn get_estimate(&self) -> f64;
}
/// State for timeouts, especially tailored toward uploading files.
/// But can be useful in any case where the size of an operation in bytes is known.
pub struct TimeoutState {
    seconds_per_unit_estimate: f64,
    cfg: AlgorithmConfig,
    specific: SpecificTimings,
}
impl TimeoutState {
    pub fn new(cfg: AlgorithmConfig, specific: SpecificTimings) -> TimeoutState {
        TimeoutState {
            seconds_per_unit_estimate: specific.seconds_per_unit,
            cfg,
            specific,
        }
    }
}
impl Timeout for TimeoutState {
    /// Not used by algorithm
    fn get_estimate(&self) -> f64 {
        self.seconds_per_unit_estimate
    }
    fn get_timeout(&self, size: usize, retries: usize) -> Duration {
        let backoff = self.cfg.backoff.powi(retries as i32);
        let time_estimate = (size as f64) * self.seconds_per_unit_estimate * backoff;
        Duration::from_secs_f64(
            self.cfg.base_timeout * backoff + self.cfg.timeout_fraction * time_estimate,
        )
    }
    fn update(&mut self, result: &RequestReport) {
        if result.size > self.specific.minimum_units_for_estimation {
            let target = result.success_time.as_secs_f64() / (result.size as f64);
            self.seconds_per_unit_estimate = self.cfg.avg_power * self.seconds_per_unit_estimate
                + (1.0 - self.cfg.avg_power) * target;
        }
    }
}
