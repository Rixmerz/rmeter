use std::sync::Mutex;

use rmeter_core::error::RmeterError;
use rmeter_core::http::client::HttpClient;
use rmeter_core::http::history::{HistoryEntry, RequestHistory};
use rmeter_core::http::request::SendRequestInput;
use rmeter_core::http::response::SendRequestOutput;

/// Send a single HTTP request using the shared connection-pool client.
///
/// The result is automatically appended to the in-memory request history.
///
/// # Errors
///
/// Returns a serialized [`RmeterError`] on failure (network errors, invalid
/// input, authentication failures, etc.).
#[tauri::command]
pub async fn send_request(
    input: SendRequestInput,
    client: tauri::State<'_, HttpClient>,
    history: tauri::State<'_, Mutex<RequestHistory>>,
) -> Result<SendRequestOutput, RmeterError> {
    let output = client.send(&input).await?;

    // Best-effort history recording — a poisoned mutex must not crash the app.
    if let Ok(mut h) = history.lock() {
        h.add(input, output.clone());
    }

    Ok(output)
}

/// Return the full in-memory request history in chronological order.
///
/// # Errors
///
/// Returns [`RmeterError::Internal`] when the history mutex is poisoned.
#[tauri::command]
pub fn get_request_history(
    history: tauri::State<'_, Mutex<RequestHistory>>,
) -> Result<Vec<HistoryEntry>, RmeterError> {
    let h = history
        .lock()
        .map_err(|e| RmeterError::Internal(format!("History mutex poisoned: {e}")))?;

    Ok(h.list().to_vec())
}

/// Clear all entries from the in-memory request history.
///
/// # Errors
///
/// Returns [`RmeterError::Internal`] when the history mutex is poisoned.
#[tauri::command]
pub fn clear_request_history(
    history: tauri::State<'_, Mutex<RequestHistory>>,
) -> Result<(), RmeterError> {
    let mut h = history
        .lock()
        .map_err(|e| RmeterError::Internal(format!("History mutex poisoned: {e}")))?;

    h.clear();
    Ok(())
}
