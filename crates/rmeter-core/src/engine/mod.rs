use serde::{Deserialize, Serialize};

pub mod aggregator;
pub mod executor;
pub mod virtual_user;

pub use aggregator::{AggregatorSnapshot, BucketStats, StreamingAggregator, TimeBucketEntry};
pub use executor::{CsvDataSet, EngineConfig, EngineEvent, EngineHandle, run_test};

/// Current operational status of the test engine.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngineStatus {
    /// Engine is idle and waiting for a plan to execute.
    #[default]
    Idle,
    /// Engine is actively running a test plan.
    Running,
    /// Engine has been signalled to stop but has not yet finished.
    Stopping,
    /// Engine has completed execution of the test plan.
    Completed,
    /// Engine encountered a fatal error during execution.
    Error,
}

impl std::fmt::Display for EngineStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EngineStatus::Idle => "idle",
            EngineStatus::Running => "running",
            EngineStatus::Stopping => "stopping",
            EngineStatus::Completed => "completed",
            EngineStatus::Error => "error",
        };
        write!(f, "{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_status_is_idle() {
        let status = EngineStatus::default();
        assert_eq!(status, EngineStatus::Idle);
    }

    #[test]
    fn display_idle() {
        assert_eq!(EngineStatus::Idle.to_string(), "idle");
    }

    #[test]
    fn display_running() {
        assert_eq!(EngineStatus::Running.to_string(), "running");
    }

    #[test]
    fn display_stopping() {
        assert_eq!(EngineStatus::Stopping.to_string(), "stopping");
    }

    #[test]
    fn display_completed() {
        assert_eq!(EngineStatus::Completed.to_string(), "completed");
    }

    #[test]
    fn display_error() {
        assert_eq!(EngineStatus::Error.to_string(), "error");
    }

    #[test]
    fn clone_preserves_value() {
        let status = EngineStatus::Running;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn equality_same_variant() {
        assert_eq!(EngineStatus::Idle, EngineStatus::Idle);
        assert_eq!(EngineStatus::Running, EngineStatus::Running);
    }

    #[test]
    fn inequality_different_variants() {
        assert_ne!(EngineStatus::Idle, EngineStatus::Running);
        assert_ne!(EngineStatus::Running, EngineStatus::Completed);
        assert_ne!(EngineStatus::Stopping, EngineStatus::Error);
    }

    #[test]
    fn serialize_deserialize_roundtrip() {
        let status = EngineStatus::Running;
        let json = serde_json::to_string(&status).expect("serialize should succeed");
        assert_eq!(json, "\"running\"");
        let deserialized: EngineStatus =
            serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(deserialized, status);
    }

    #[test]
    fn deserialize_all_variants() {
        let cases = [
            ("\"idle\"", EngineStatus::Idle),
            ("\"running\"", EngineStatus::Running),
            ("\"stopping\"", EngineStatus::Stopping),
            ("\"completed\"", EngineStatus::Completed),
            ("\"error\"", EngineStatus::Error),
        ];
        for (json, expected) in cases {
            let parsed: EngineStatus =
                serde_json::from_str(json).expect("should parse");
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn deserialize_invalid_variant_fails() {
        let result = serde_json::from_str::<EngineStatus>("\"unknown\"");
        assert!(result.is_err());
    }
}
