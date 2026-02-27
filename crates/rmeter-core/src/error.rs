use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum RmeterError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Engine error: {0}")]
    Engine(String),

    #[error("Plan not found: {0}")]
    PlanNotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),
}

impl Serialize for RmeterError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_error_display() {
        let err = RmeterError::Validation("field X is required".to_string());
        assert_eq!(err.to_string(), "Validation error: field X is required");
    }

    #[test]
    fn engine_error_display() {
        let err = RmeterError::Engine("timeout".to_string());
        assert_eq!(err.to_string(), "Engine error: timeout");
    }

    #[test]
    fn plan_not_found_display() {
        let err = RmeterError::PlanNotFound("abc-123".to_string());
        assert_eq!(err.to_string(), "Plan not found: abc-123");
    }

    #[test]
    fn internal_error_display() {
        let err = RmeterError::Internal("unexpected state".to_string());
        assert_eq!(err.to_string(), "Internal error: unexpected state");
    }

    #[test]
    fn websocket_error_display() {
        let err = RmeterError::WebSocket("connection refused".to_string());
        assert_eq!(err.to_string(), "WebSocket error: connection refused");
    }

    #[test]
    fn io_error_from_std() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: RmeterError = io_err.into();
        let msg = err.to_string();
        assert!(msg.contains("IO error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn serde_error_from_json() {
        let json_err = serde_json::from_str::<serde_json::Value>("not valid json").unwrap_err();
        let err: RmeterError = json_err.into();
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn serialize_produces_string() {
        let err = RmeterError::Validation("test error".to_string());
        let json = serde_json::to_string(&err).expect("serialize should succeed");
        assert_eq!(json, "\"Validation error: test error\"");
    }

    #[test]
    fn serialize_engine_error() {
        let err = RmeterError::Engine("engine failed".to_string());
        let json = serde_json::to_string(&err).expect("serialize should succeed");
        assert_eq!(json, "\"Engine error: engine failed\"");
    }

    #[test]
    fn error_is_debug() {
        let err = RmeterError::Validation("test".to_string());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Validation"));
    }
}
