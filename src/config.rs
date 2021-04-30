use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Maximum number of simultaneous upload requests
    pub copy_parallelization: usize,

    pub algorithm: AlgorithmConfig,

    pub delete_requests: SpecificTimings,

    /// NOTE: For now, `put_request` is used both in S3 `get`, `put` and `copy` operations.
    /// Reason: We don't know if it's worth it with different configurations for these operations
    /// that all have a duration that depends on the number of bytes of the objects in question.
    pub put_requests: SpecificTimings,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            copy_parallelization: 20,
            algorithm: Default::default(),
            delete_requests: SpecificTimings {
                seconds_per_unit: 0.2,
                minimum_units_for_estimation: 10,
            },
            put_requests: SpecificTimings {
                seconds_per_unit: 1.0 / 1_000_000.0, // 1 MBPS = 1e-06 seconds per MB
                minimum_units_for_estimation: 10,
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlgorithmConfig {
    /// The base timeout which will always be there (an estimate of RTT)
    pub base_timeout: f64,

    /// Timeout is set to a fraction of expected upload time (> 1.0)
    pub timeout_fraction: f64,

    /// Every retry, the timeout is multiplied by backoff (> 1.0)
    pub backoff: f64,

    /// Number of times to retry a single request before giving up
    pub n_retries: usize,

    /// To estimate the upload speed incrementally, we use an exponential average:
    /// `new_avg_speed = avg_power * new_speed + (1 - avg_power) * avg_speed`.
    ///
    /// Thus, between 0.0 and 1.0, closer to 1.0 means that newer data points have
    /// more significance.
    pub avg_power: f64,
}
impl Default for AlgorithmConfig {
    fn default() -> Self {
        Self {
            base_timeout: 0.5,
            timeout_fraction: 1.5,
            backoff: 1.5,
            n_retries: 8,
            // expected_upload_speed: 1.0,
            avg_power: 0.7,
            // avg_min_bytes: 1_000_000,
        }
    }
}

/// These settings are specific to the kind of operation we do. For example delete or put in S3.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecificTimings {
    /// The initial estimate of extra timeout per unit (byte or object)
    pub seconds_per_unit: f64,
    /// The amount of units in a request, below which it does not affect estimation
    pub minimum_units_for_estimation: u64,
}

impl SpecificTimings {
    /// Sane default setting for when the size is number of bytes
    pub fn default_for_bytes() -> Self {
        Self {
            seconds_per_unit: 1.0 / 1_000_000.0,   // 1 MBPS
            minimum_units_for_estimation: 500_000, // 500 KB
        }
    }
    /// Sane default setting for when the size is number of objects
    pub fn default_for_objects() -> Self {
        Self {
            seconds_per_unit: 0.2,
            minimum_units_for_estimation: 2,
        }
    }
}

// DRAFT
//
// Now, we don't have "avg_min_bytes". Because... we will just substract the assumed constant
// anyway.
// What if the assumption is wrong?
// Well, it should be rather small anyway. It is exclusively thought to be the RTT...
// If the substraction is negative after all...? Then... idk

// put_timeout_per_byte..?
//  should configure it as an assumed MBPS just like before. expected_upload_speed.
// delete_timeout_per_object..?
//  quite straight-forward seconds per object
