use serde::{Deserialize, Serialize};

use crate::http::request::SendRequestInput;
use crate::http::response::SendRequestOutput;

/// A single entry in the request history log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique identifier for this history entry.
    pub id: String,
    /// The original request input.
    pub input: SendRequestInput,
    /// The response received for that request.
    pub output: SendRequestOutput,
    /// RFC 3339 timestamp of when the request was executed.
    pub timestamp: String,
}

/// In-memory bounded ring-buffer of completed HTTP requests.
///
/// Entries are appended in chronological order. When the buffer is full,
/// the oldest entry is dropped to make room for the new one.
pub struct RequestHistory {
    entries: Vec<HistoryEntry>,
    max_entries: usize,
}

impl RequestHistory {
    /// Create a new history store that keeps at most `max_entries` entries.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Vec::new(),
            max_entries,
        }
    }

    /// Record a completed request/response pair and return the generated entry id.
    pub fn add(&mut self, input: SendRequestInput, output: SendRequestOutput) -> String {
        let id = uuid::Uuid::new_v4().to_string();
        let timestamp = chrono::Utc::now().to_rfc3339();

        let entry = HistoryEntry {
            id: id.clone(),
            input,
            output,
            timestamp,
        };

        self.entries.push(entry);

        // Drop the oldest entry if we are over the limit.
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        id
    }

    /// Return a slice of all stored entries in chronological order.
    pub fn list(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Remove all stored entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::request::SendRequestInput;
    use crate::http::response::SendRequestOutput;
    use crate::plan::model::HttpMethod;
    use std::collections::HashMap;

    fn make_input(url: &str) -> SendRequestInput {
        SendRequestInput {
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            auth: None,
        }
    }

    fn make_output(status: u16) -> SendRequestOutput {
        SendRequestOutput {
            status,
            headers: HashMap::new(),
            body: String::new(),
            elapsed_ms: 42,
            size_bytes: 0,
        }
    }

    #[test]
    fn new_history_is_empty() {
        let history = RequestHistory::new(10);
        assert!(history.list().is_empty());
    }

    #[test]
    fn add_returns_unique_ids() {
        let mut history = RequestHistory::new(10);
        let id1 = history.add(make_input("http://a.com"), make_output(200));
        let id2 = history.add(make_input("http://b.com"), make_output(201));
        assert_ne!(id1, id2);
    }

    #[test]
    fn add_inserts_entry_in_list() {
        let mut history = RequestHistory::new(10);
        let id = history.add(make_input("http://example.com"), make_output(200));
        let entries = history.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        assert_eq!(entries[0].input.url, "http://example.com");
        assert_eq!(entries[0].output.status, 200);
    }

    #[test]
    fn list_preserves_insertion_order() {
        let mut history = RequestHistory::new(10);
        history.add(make_input("http://first.com"), make_output(200));
        history.add(make_input("http://second.com"), make_output(201));
        history.add(make_input("http://third.com"), make_output(202));
        let entries = history.list();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].input.url, "http://first.com");
        assert_eq!(entries[1].input.url, "http://second.com");
        assert_eq!(entries[2].input.url, "http://third.com");
    }

    #[test]
    fn respects_max_entries_limit() {
        let mut history = RequestHistory::new(3);
        for i in 0..5 {
            history.add(make_input(&format!("http://url{i}.com")), make_output(200));
        }
        let entries = history.list();
        assert_eq!(entries.len(), 3);
        // Oldest two should have been evicted
        assert_eq!(entries[0].input.url, "http://url2.com");
        assert_eq!(entries[1].input.url, "http://url3.com");
        assert_eq!(entries[2].input.url, "http://url4.com");
    }

    #[test]
    fn max_entries_one() {
        let mut history = RequestHistory::new(1);
        history.add(make_input("http://a.com"), make_output(200));
        history.add(make_input("http://b.com"), make_output(201));
        let entries = history.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].input.url, "http://b.com");
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut history = RequestHistory::new(10);
        history.add(make_input("http://example.com"), make_output(200));
        history.add(make_input("http://example.com"), make_output(201));
        assert_eq!(history.list().len(), 2);
        history.clear();
        assert!(history.list().is_empty());
    }

    #[test]
    fn entry_has_valid_timestamp() {
        let mut history = RequestHistory::new(10);
        history.add(make_input("http://example.com"), make_output(200));
        let entry = &history.list()[0];
        // Timestamp should be a valid RFC 3339 string (parseable by chrono)
        assert!(chrono::DateTime::parse_from_rfc3339(&entry.timestamp).is_ok());
    }

    #[test]
    fn entry_has_valid_uuid_id() {
        let mut history = RequestHistory::new(10);
        history.add(make_input("http://example.com"), make_output(200));
        let entry = &history.list()[0];
        assert!(uuid::Uuid::parse_str(&entry.id).is_ok());
    }

    #[test]
    fn add_after_clear_works() {
        let mut history = RequestHistory::new(5);
        history.add(make_input("http://a.com"), make_output(200));
        history.clear();
        history.add(make_input("http://b.com"), make_output(201));
        let entries = history.list();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].input.url, "http://b.com");
    }
}
