//! Shared helpers for authenticated calls from the Tauri host to the local core RPC.

use std::time::Duration;

use reqwest::RequestBuilder;
use serde::Serialize;

const CORE_RPC_URL_ENV: &str = "OPENHUMAN_CORE_RPC_URL";
pub(crate) fn core_rpc_url_value() -> String {
    std::env::var(CORE_RPC_URL_ENV).unwrap_or_else(|_| {
        format!(
            "http://127.0.0.1:{}/rpc",
            crate::core_process::default_core_port()
        )
    })
}

pub(crate) fn apply_auth(builder: RequestBuilder) -> Result<RequestBuilder, String> {
    let token = crate::core_process::current_rpc_token()
        .ok_or_else(|| "core RPC token is not initialized".to_string())?;
    Ok(builder.header("Authorization", format!("Bearer {token}")))
}

/// Verbatim status + body from an upstream runtime, mirrored back to the
/// renderer so it can reuse its existing JSON-RPC envelope parsing.
#[derive(Serialize)]
pub(crate) struct RelayHttpResponse {
    pub status: u16,
    pub body: String,
}

/// Normalize an optional bearer token into the header value to send, if any.
/// A `None` or blank/whitespace token yields `None` so we never emit an empty
/// `Authorization` header (local OpenAI-compatible runtimes that need no auth
/// would otherwise see a malformed bearer).
fn relay_bearer_header(token: Option<&str>) -> Option<String> {
    token
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| format!("Bearer {t}"))
}

/// POST a JSON-RPC body to an arbitrary self-hosted runtime URL from the Rust
/// host instead of the webview.
///
/// Why this exists (#3865): the desktop webview origin is `tauri://localhost`,
/// a *secure context*. Chromium treats `http://127.0.0.1` / `localhost` as
/// "potentially trustworthy", so browser `fetch()` to the embedded local core
/// works — but a self-hosted runtime on a LAN IP (e.g.
/// `http://192.168.1.74:7788`) is plain cleartext from a secure context, so the
/// fetch is blocked as mixed content before any request leaves the browser
/// ("Failed to fetch", and the runtime never logs a `/rpc` hit) even though the
/// endpoint is healthy and reachable from curl/Safari. Issuing the request from
/// the Rust host with `reqwest` bypasses the webview's mixed-content / CORS
/// restrictions entirely — the same way the shell already talks to the local
/// core.
///
/// Returns the upstream status + body verbatim (including JSON-RPC error
/// envelopes and any 4xx/5xx) so the renderer keeps its existing handling; only
/// transport-level failures (DNS, connect, timeout) surface as `Err`.
#[tauri::command]
pub(crate) async fn relay_http_rpc(
    url: String,
    token: Option<String>,
    body: String,
) -> Result<RelayHttpResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| format!("failed to build HTTP client: {e}"))?;

    let mut builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .body(body);

    let bearer = relay_bearer_header(token.as_deref());
    if let Some(value) = bearer.as_deref() {
        builder = builder.header("Authorization", value);
    }

    log::debug!("[core_rpc][relay] POST {url} (auth={})", bearer.is_some());

    let resp = builder
        .send()
        .await
        .map_err(|e| format!("request to {url} failed: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp
        .text()
        .await
        .map_err(|e| format!("failed to read response body from {url}: {e}"))?;
    log::debug!(
        "[core_rpc][relay] ← {url} status={status} body_len={}",
        text.len()
    );
    Ok(RelayHttpResponse { status, body: text })
}

#[cfg(test)]
mod tests {
    use super::relay_bearer_header;

    #[test]
    fn bearer_header_present_for_real_token() {
        assert_eq!(
            relay_bearer_header(Some("tok123")).as_deref(),
            Some("Bearer tok123")
        );
        // Surrounding whitespace is trimmed before formatting.
        assert_eq!(
            relay_bearer_header(Some("  tok123  ")).as_deref(),
            Some("Bearer tok123")
        );
    }

    #[test]
    fn bearer_header_absent_for_missing_or_blank_token() {
        assert_eq!(relay_bearer_header(None), None);
        assert_eq!(relay_bearer_header(Some("")), None);
        assert_eq!(relay_bearer_header(Some("   ")), None);
    }
}
