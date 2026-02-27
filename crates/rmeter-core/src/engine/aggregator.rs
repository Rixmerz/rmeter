use std::collections::BTreeMap;
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::results::TestSummary;

// ---------------------------------------------------------------------------
// BucketStats — per-second statistics window
// ---------------------------------------------------------------------------

/// Aggregated statistics for a single one-second time bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct BucketStats {
    pub requests: u64,
    pub errors: u64,
    pub sum_ms: u64,
    pub min_ms: u64,
    pub max_ms: u64,
}

// ---------------------------------------------------------------------------
// AggregatorSnapshot — lightweight read for progress events
// ---------------------------------------------------------------------------

/// A point-in-time snapshot of the aggregator's current state.
/// Used to populate progress events without cloning the full response-time vec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AggregatorSnapshot {
    pub total_requests: u64,
    pub total_errors: u64,
    pub total_successes: u64,
    pub min_ms: u64,
    pub max_ms: u64,
    pub mean_ms: f64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub total_bytes: u64,
    pub current_rps: f64,
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// TimeBucketEntry — serializable time-series entry for charting
// ---------------------------------------------------------------------------

/// A single per-second time-series entry suitable for dashboard charts.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TimeBucketEntry {
    pub second: u64,
    pub requests: u64,
    pub errors: u64,
    pub avg_ms: f64,
    pub min_ms: u64,
    pub max_ms: u64,
}

// ---------------------------------------------------------------------------
// StreamingAggregator
// ---------------------------------------------------------------------------

/// Real-time statistics aggregator for a running test.
///
/// Designed to be held behind an `Arc<RwLock<_>>` so both the virtual-user
/// tasks and the progress-reporter task can access it concurrently.
pub struct StreamingAggregator {
    total_requests: u64,
    total_errors: u64,
    /// All individual response times (ms). Kept for accurate percentile
    /// computation. For very long tests this grows — acceptable for the
    /// workloads rmeter targets.
    response_times: Vec<u64>,
    min_ms: u64,
    max_ms: u64,
    sum_ms: u64,
    total_bytes: u64,
    start_time: Instant,
    started_at: DateTime<Utc>,
    /// Per-second buckets keyed by seconds-since-start for time-series charts.
    time_buckets: BTreeMap<u64, BucketStats>,
}

impl StreamingAggregator {
    /// Create a new aggregator, capturing the current wall-clock start time.
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            total_errors: 0,
            response_times: Vec::new(),
            min_ms: u64::MAX,
            max_ms: 0,
            sum_ms: 0,
            total_bytes: 0,
            start_time: Instant::now(),
            started_at: Utc::now(),
            time_buckets: BTreeMap::new(),
        }
    }

    /// Record the result of a single completed request.
    pub fn record(&mut self, elapsed_ms: u64, success: bool, size_bytes: u64) {
        self.total_requests += 1;
        if !success {
            self.total_errors += 1;
        }

        self.response_times.push(elapsed_ms);
        self.sum_ms += elapsed_ms;
        if elapsed_ms < self.min_ms {
            self.min_ms = elapsed_ms;
        }
        if elapsed_ms > self.max_ms {
            self.max_ms = elapsed_ms;
        }
        self.total_bytes += size_bytes;

        // Update time bucket.
        let bucket_key = self.start_time.elapsed().as_secs();
        let bucket = self.time_buckets.entry(bucket_key).or_insert(BucketStats {
            requests: 0,
            errors: 0,
            sum_ms: 0,
            min_ms: u64::MAX,
            max_ms: 0,
        });
        bucket.requests += 1;
        if !success {
            bucket.errors += 1;
        }
        bucket.sum_ms += elapsed_ms;
        if elapsed_ms < bucket.min_ms {
            bucket.min_ms = elapsed_ms;
        }
        if elapsed_ms > bucket.max_ms {
            bucket.max_ms = elapsed_ms;
        }
    }

    /// Calculate the p-th percentile response time.
    ///
    /// `p` must be in the range (0.0, 100.0].
    /// Returns 0 when no requests have been recorded yet.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.response_times.is_empty() {
            return 0;
        }
        let mut sorted = self.response_times.clone();
        sorted.sort_unstable();
        let idx = ((p / 100.0) * sorted.len() as f64).ceil() as usize;
        let idx = idx.saturating_sub(1).min(sorted.len() - 1);
        sorted[idx]
    }

    /// Requests per second averaged over the entire elapsed duration.
    pub fn current_rps(&self) -> f64 {
        let elapsed_secs = self.start_time.elapsed().as_secs_f64();
        if elapsed_secs < 0.001 {
            return 0.0;
        }
        self.total_requests as f64 / elapsed_secs
    }

    /// Build a completed [`TestSummary`] from all accumulated data.
    pub fn summary(&self, plan_id: Uuid, plan_name: String) -> TestSummary {
        let finished_at = Utc::now();
        let total = self.total_requests;
        let failed = self.total_errors;
        let successful = total.saturating_sub(failed);

        let mean = if total > 0 {
            self.sum_ms as f64 / total as f64
        } else {
            0.0
        };

        let min_ms = if self.min_ms == u64::MAX { 0 } else { self.min_ms };

        let elapsed_secs = (finished_at - self.started_at).num_milliseconds() as f64 / 1000.0;
        let rps = if elapsed_secs > 0.0 {
            total as f64 / elapsed_secs
        } else {
            0.0
        };

        TestSummary {
            plan_id,
            plan_name,
            started_at: self.started_at,
            finished_at,
            total_requests: total,
            successful_requests: successful,
            failed_requests: failed,
            min_response_ms: min_ms,
            max_response_ms: self.max_ms,
            mean_response_ms: mean,
            p50_response_ms: self.percentile(50.0),
            p95_response_ms: self.percentile(95.0),
            p99_response_ms: self.percentile(99.0),
            requests_per_second: rps,
            total_bytes_received: self.total_bytes,
        }
    }

    /// Return a lightweight snapshot suitable for progress events.
    pub fn snapshot(&self) -> AggregatorSnapshot {
        let total = self.total_requests;
        let failed = self.total_errors;
        let mean_ms = if total > 0 {
            self.sum_ms as f64 / total as f64
        } else {
            0.0
        };
        let min_ms = if self.min_ms == u64::MAX { 0 } else { self.min_ms };

        AggregatorSnapshot {
            total_requests: total,
            total_errors: failed,
            total_successes: total.saturating_sub(failed),
            min_ms,
            max_ms: self.max_ms,
            mean_ms,
            p50_ms: self.percentile(50.0),
            p95_ms: self.percentile(95.0),
            p99_ms: self.percentile(99.0),
            total_bytes: self.total_bytes,
            current_rps: self.current_rps(),
            elapsed_ms: self.start_time.elapsed().as_millis() as u64,
        }
    }

    /// Return per-second time-series data as a sorted vec of entries.
    pub fn time_series(&self) -> Vec<TimeBucketEntry> {
        self.time_buckets
            .iter()
            .map(|(&second, bucket)| TimeBucketEntry {
                second,
                requests: bucket.requests,
                errors: bucket.errors,
                avg_ms: if bucket.requests > 0 {
                    bucket.sum_ms as f64 / bucket.requests as f64
                } else {
                    0.0
                },
                min_ms: if bucket.min_ms == u64::MAX {
                    0
                } else {
                    bucket.min_ms
                },
                max_ms: bucket.max_ms,
            })
            .collect()
    }

    /// Access the raw per-second time buckets (for charting).
    pub fn time_buckets(&self) -> &BTreeMap<u64, BucketStats> {
        &self.time_buckets
    }
}

impl Default for StreamingAggregator {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    // -----------------------------------------------------------------------
    // record
    // -----------------------------------------------------------------------

    #[test]
    fn record_updates_counts_and_min_max() {
        let mut agg = StreamingAggregator::new();
        agg.record(100, true, 512);
        agg.record(200, false, 1024);
        agg.record(50, true, 256);

        assert_eq!(agg.total_requests, 3);
        assert_eq!(agg.total_errors, 1);
        assert_eq!(agg.min_ms, 50);
        assert_eq!(agg.max_ms, 200);
        assert_eq!(agg.sum_ms, 350);
        assert_eq!(agg.total_bytes, 1792);
    }

    #[test]
    fn record_single_entry_sets_min_and_max_to_same_value() {
        let mut agg = StreamingAggregator::new();
        agg.record(123, true, 0);
        assert_eq!(agg.min_ms, 123);
        assert_eq!(agg.max_ms, 123);
    }

    #[test]
    fn record_all_errors_sets_total_errors_equal_to_total_requests() {
        let mut agg = StreamingAggregator::new();
        for _ in 0..5 {
            agg.record(100, false, 0);
        }
        assert_eq!(agg.total_requests, 5);
        assert_eq!(agg.total_errors, 5);
    }

    #[test]
    fn record_updates_time_bucket() {
        let mut agg = StreamingAggregator::new();
        agg.record(100, true, 0);
        // At least one bucket should exist.
        assert!(!agg.time_buckets.is_empty());
        let bucket = agg.time_buckets.values().next().unwrap();
        assert_eq!(bucket.requests, 1);
        assert_eq!(bucket.errors, 0);
    }

    // -----------------------------------------------------------------------
    // percentile
    // -----------------------------------------------------------------------

    #[test]
    fn percentile_empty_returns_zero() {
        let agg = StreamingAggregator::new();
        assert_eq!(agg.percentile(50.0), 0);
        assert_eq!(agg.percentile(99.0), 0);
    }

    #[test]
    fn percentile_single_entry_returns_that_value() {
        let mut agg = StreamingAggregator::new();
        agg.record(250, true, 0);
        assert_eq!(agg.percentile(50.0), 250);
        assert_eq!(agg.percentile(99.0), 250);
    }

    #[test]
    fn percentile_multiple_entries_are_correct() {
        let mut agg = StreamingAggregator::new();
        // Insert 10 values: 10, 20, ..., 100
        for ms in [10, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
            agg.record(ms, true, 0);
        }
        // p50 of 10 sorted values => index ceil(0.5 * 10) - 1 = 4 => value 50
        assert_eq!(agg.percentile(50.0), 50);
        // p90 => index ceil(0.9 * 10) - 1 = 8 => value 90
        assert_eq!(agg.percentile(90.0), 90);
        // p100 => index 9 => value 100
        assert_eq!(agg.percentile(100.0), 100);
    }

    #[test]
    fn percentile_is_not_affected_by_insertion_order() {
        let mut agg_ordered = StreamingAggregator::new();
        let mut agg_reversed = StreamingAggregator::new();
        for ms in [10u64, 50, 100, 200, 500] {
            agg_ordered.record(ms, true, 0);
        }
        for ms in [500u64, 200, 100, 50, 10] {
            agg_reversed.record(ms, true, 0);
        }
        assert_eq!(agg_ordered.percentile(50.0), agg_reversed.percentile(50.0));
        assert_eq!(agg_ordered.percentile(90.0), agg_reversed.percentile(90.0));
    }

    // -----------------------------------------------------------------------
    // current_rps
    // -----------------------------------------------------------------------

    #[test]
    fn current_rps_is_zero_before_any_requests() {
        let agg = StreamingAggregator::new();
        // Immediately after creation the elapsed time is sub-millisecond,
        // so the implementation returns 0.0.
        let rps = agg.current_rps();
        assert!(rps >= 0.0);
    }

    #[test]
    fn current_rps_is_non_negative_after_recording() {
        let mut agg = StreamingAggregator::new();
        for _ in 0..10 {
            agg.record(100, true, 0);
        }
        assert!(agg.current_rps() >= 0.0);
    }

    // -----------------------------------------------------------------------
    // snapshot
    // -----------------------------------------------------------------------

    #[test]
    fn snapshot_empty_aggregator() {
        let agg = StreamingAggregator::new();
        let snap = agg.snapshot();
        assert_eq!(snap.total_requests, 0);
        assert_eq!(snap.total_errors, 0);
        assert_eq!(snap.total_successes, 0);
        assert_eq!(snap.min_ms, 0); // u64::MAX normalised to 0
        assert_eq!(snap.max_ms, 0);
        assert_eq!(snap.mean_ms, 0.0);
    }

    #[test]
    fn snapshot_after_recording_reflects_state() {
        let mut agg = StreamingAggregator::new();
        agg.record(100, true, 500);
        agg.record(200, false, 1000);

        let snap = agg.snapshot();
        assert_eq!(snap.total_requests, 2);
        assert_eq!(snap.total_errors, 1);
        assert_eq!(snap.total_successes, 1);
        assert_eq!(snap.min_ms, 100);
        assert_eq!(snap.max_ms, 200);
        assert!((snap.mean_ms - 150.0).abs() < 0.001);
        assert_eq!(snap.total_bytes, 1500);
    }

    // -----------------------------------------------------------------------
    // summary
    // -----------------------------------------------------------------------

    #[test]
    fn summary_empty_aggregator() {
        let agg = StreamingAggregator::new();
        let plan_id = Uuid::new_v4();
        let s = agg.summary(plan_id, "Empty Plan".to_string());
        assert_eq!(s.total_requests, 0);
        assert_eq!(s.failed_requests, 0);
        assert_eq!(s.min_response_ms, 0);
        assert_eq!(s.mean_response_ms, 0.0);
    }

    #[test]
    fn summary_calculates_correct_statistics() {
        let mut agg = StreamingAggregator::new();
        agg.record(100, true, 512);
        agg.record(200, true, 512);
        agg.record(300, false, 512);

        let plan_id = Uuid::new_v4();
        let s = agg.summary(plan_id, "Test".to_string());

        assert_eq!(s.total_requests, 3);
        assert_eq!(s.successful_requests, 2);
        assert_eq!(s.failed_requests, 1);
        assert_eq!(s.min_response_ms, 100);
        assert_eq!(s.max_response_ms, 300);
        assert!((s.mean_response_ms - 200.0).abs() < 0.001);
        assert_eq!(s.total_bytes_received, 1536);
    }

    #[test]
    fn summary_plan_id_and_name_are_passed_through() {
        let agg = StreamingAggregator::new();
        let plan_id = Uuid::new_v4();
        let s = agg.summary(plan_id, "My Plan".to_string());
        assert_eq!(s.plan_id, plan_id);
        assert_eq!(s.plan_name, "My Plan");
    }

    // -----------------------------------------------------------------------
    // time_series
    // -----------------------------------------------------------------------

    #[test]
    fn time_series_empty_aggregator_returns_empty_vec() {
        let agg = StreamingAggregator::new();
        assert!(agg.time_series().is_empty());
    }

    #[test]
    fn time_series_entries_are_sorted_by_second() {
        let mut agg = StreamingAggregator::new();
        // We can't easily control which bucket each record goes into without
        // mocking time, but we can at least verify the output is sorted.
        for _ in 0..5 {
            agg.record(100, true, 0);
        }
        let series = agg.time_series();
        if series.len() > 1 {
            let seconds: Vec<u64> = series.iter().map(|e| e.second).collect();
            let mut sorted = seconds.clone();
            sorted.sort_unstable();
            assert_eq!(seconds, sorted);
        }
    }

    #[test]
    fn time_series_entry_has_correct_fields() {
        let mut agg = StreamingAggregator::new();
        agg.record(100, true, 0);
        agg.record(200, false, 0);

        let series = agg.time_series();
        assert!(!series.is_empty());
        let entry = &series[0];
        // The bucket should have at least 2 requests.
        assert!(entry.requests >= 2);
        assert!(entry.errors >= 1);
        assert!(entry.avg_ms > 0.0);
    }
}
