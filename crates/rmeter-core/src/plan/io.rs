use std::path::Path;

use crate::error::RmeterError;
use crate::plan::model::TestPlan;

/// Read a `.rmeter` plan file from disk.
///
/// The file format is JSON serialized [`TestPlan`].
pub async fn read_plan(path: impl AsRef<Path>) -> Result<TestPlan, RmeterError> {
    let content = tokio::fs::read_to_string(path.as_ref()).await?;
    let plan: TestPlan = serde_json::from_str(&content)?;
    Ok(plan)
}

/// Write a [`TestPlan`] to a `.rmeter` file on disk.
///
/// The plan is serialized as pretty-printed JSON for human readability.
pub async fn write_plan(plan: &TestPlan, path: impl AsRef<Path>) -> Result<(), RmeterError> {
    let content = serde_json::to_string_pretty(plan)?;
    tokio::fs::write(path.as_ref(), content).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::model::LoopCount;

    /// Build a minimal but complete [`TestPlan`] for round-trip testing.
    fn make_test_plan() -> TestPlan {
        use std::collections::HashMap;
        use crate::plan::model::{HttpMethod, HttpRequest, ThreadGroup};
        use uuid::Uuid;

        let req = HttpRequest {
            id: Uuid::new_v4(),
            name: "Health Check".to_string(),
            method: HttpMethod::Get,
            url: "https://example.com/health".to_string(),
            headers: HashMap::new(),
            body: None,
            assertions: Vec::new(),
            extractors: Vec::new(),
            enabled: true,
        };

        let tg = ThreadGroup {
            id: Uuid::new_v4(),
            name: "Thread Group 1".to_string(),
            num_threads: 10,
            ramp_up_seconds: 5,
            loop_count: LoopCount::Finite { count: 100 },
            requests: vec![req],
            enabled: true,
        };

        TestPlan {
            id: Uuid::new_v4(),
            name: "Round-Trip Plan".to_string(),
            description: "A plan for serialization testing".to_string(),
            thread_groups: vec![tg],
            variables: Vec::new(),
            csv_data_sources: Vec::new(),
            format_version: 1,
        }
    }

    #[tokio::test]
    async fn round_trip_write_then_read_preserves_plan() {
        let plan = make_test_plan();
        let dir = tempfile::tempdir().expect("tempdir should be created");
        // Safety: tempdir exists and is writable; unwrap is acceptable in tests.
        let path = dir.path().join("test_plan.rmeter");

        write_plan(&plan, &path).await.expect("write_plan should succeed");
        let loaded = read_plan(&path).await.expect("read_plan should succeed");

        assert_eq!(loaded.id, plan.id);
        assert_eq!(loaded.name, plan.name);
        assert_eq!(loaded.description, plan.description);
        assert_eq!(loaded.format_version, plan.format_version);
        assert_eq!(loaded.thread_groups.len(), plan.thread_groups.len());

        let orig_tg = &plan.thread_groups[0];
        let load_tg = &loaded.thread_groups[0];
        assert_eq!(load_tg.id, orig_tg.id);
        assert_eq!(load_tg.name, orig_tg.name);
        assert_eq!(load_tg.num_threads, orig_tg.num_threads);
        assert_eq!(load_tg.ramp_up_seconds, orig_tg.ramp_up_seconds);
        assert!(matches!(load_tg.loop_count, LoopCount::Finite { count: 100 }));

        let orig_req = &orig_tg.requests[0];
        let load_req = &load_tg.requests[0];
        assert_eq!(load_req.id, orig_req.id);
        assert_eq!(load_req.name, orig_req.name);
        assert_eq!(load_req.url, orig_req.url);
    }

    #[tokio::test]
    async fn read_plan_error_for_nonexistent_file() {
        let result = read_plan("/nonexistent/path/plan.rmeter").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_plan_error_for_invalid_json() {
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("bad.rmeter");
        // Safety: write is straightforward in a test temp dir.
        tokio::fs::write(&path, b"not valid json at all")
            .await
            .expect("writing bad file should succeed");
        let result = read_plan(&path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn write_plan_succeeds_to_valid_path() {
        let plan = make_test_plan();
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("output.rmeter");
        let result = write_plan(&plan, &path).await;
        assert!(result.is_ok());
        // File should actually exist and contain JSON.
        let content = tokio::fs::read_to_string(&path).await.expect("file should be readable");
        assert!(content.contains(&plan.name));
        assert!(content.contains("thread_groups"));
    }

    #[tokio::test]
    async fn write_plan_produces_pretty_json() {
        let plan = make_test_plan();
        let dir = tempfile::tempdir().expect("tempdir should be created");
        let path = dir.path().join("pretty.rmeter");
        write_plan(&plan, &path).await.expect("write should succeed");
        let content = tokio::fs::read_to_string(&path).await.expect("file should be readable");
        // Pretty-printed JSON contains newlines and indentation.
        assert!(content.contains('\n'));
        assert!(content.contains("  "));
    }
}
