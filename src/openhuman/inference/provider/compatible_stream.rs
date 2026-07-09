//! SSE streaming support for the OpenAI-compatible provider.
//!
//! Converts a raw `reqwest::Response` byte stream into a typed
//! `StreamChunk` stream via Server-Sent Events parsing.

use crate::openhuman::inference::provider::traits::{StreamChunk, StreamError, StreamResult};
use futures_util::{stream, Stream, StreamExt};
use std::time::Duration;

use super::compatible_parse::parse_sse_line;

/// Default mid-stream idle timeout: if the upstream sends no bytes for this long
/// *after the stream has been established*, we treat the connection as stalled
/// and emit a terminal [`StreamError::IdleTimeout`] instead of blocking on
/// `bytes_stream.next()` forever (#4761 — a stalled upstream previously orphaned
/// the turn with no `chat_done`/`chat_error`). Generous enough to tolerate a slow
/// first token / long silent reasoning gap on staging, but far below the ~250s
/// point where the connection was being dropped with no terminal event.
///
/// Override with `OPENHUMAN_INFERENCE_STREAM_IDLE_TIMEOUT_SECS`; set it to `0` to
/// disable the guard entirely (restores the pre-#4761 unbounded wait).
const DEFAULT_STREAM_IDLE_TIMEOUT_SECS: u64 = 120;

/// Resolve the configured mid-stream idle timeout. `None` disables the guard.
fn stream_idle_timeout() -> Option<Duration> {
    let secs = std::env::var("OPENHUMAN_INFERENCE_STREAM_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_STREAM_IDLE_TIMEOUT_SECS);
    (secs > 0).then(|| Duration::from_secs(secs))
}

/// Outcome of a single idle-bounded read from an item stream.
#[derive(Debug, PartialEq, Eq)]
enum IdleRead<T> {
    /// The stream yielded an item.
    Item(T),
    /// The stream ended cleanly (`next()` returned `None`).
    Ended,
    /// No item arrived within the idle timeout — the upstream stalled.
    IdleTimedOut,
}

/// Read the next item from `stream`, enforcing an optional idle timeout.
///
/// Extracted as a small generic helper (over the concrete
/// `reqwest::Response::bytes_stream()`) so the stall→terminal-error decision is
/// unit-testable without standing up a live HTTP response.
async fn read_next_or_idle<S, T>(stream: &mut S, idle: Option<Duration>) -> IdleRead<T>
where
    S: Stream<Item = T> + Unpin,
{
    match idle {
        Some(dur) => match tokio::time::timeout(dur, stream.next()).await {
            Ok(Some(item)) => IdleRead::Item(item),
            Ok(None) => IdleRead::Ended,
            Err(_elapsed) => IdleRead::IdleTimedOut,
        },
        None => match stream.next().await {
            Some(item) => IdleRead::Item(item),
            None => IdleRead::Ended,
        },
    }
}

/// Convert SSE byte stream to text chunks.
pub(crate) fn sse_bytes_to_chunks(
    response: reqwest::Response,
    count_tokens: bool,
) -> stream::BoxStream<'static, StreamResult<StreamChunk>> {
    // Create a channel to send chunks
    let (tx, rx) = tokio::sync::mpsc::channel::<StreamResult<StreamChunk>>(100);

    tokio::spawn(async move {
        // Buffer for incomplete lines
        let mut buffer = String::new();

        // Get response body as bytes stream
        match response.error_for_status_ref() {
            Ok(_) => {}
            Err(e) => {
                let _ = tx.send(Err(StreamError::Http(e))).await;
                return;
            }
        }

        let mut bytes_stream = response.bytes_stream();
        let idle_timeout = stream_idle_timeout();

        loop {
            let item = match read_next_or_idle(&mut bytes_stream, idle_timeout).await {
                IdleRead::Item(item) => item,
                IdleRead::Ended => break,
                IdleRead::IdleTimedOut => {
                    // The upstream established the stream but sent no bytes for
                    // `idle_timeout`. Rather than block on `next()` forever and
                    // orphan the turn with no terminal event (#4761), surface a
                    // terminal error so the consumer fires `chat_error`.
                    if let Some(dur) = idle_timeout {
                        let _ = tx.send(Err(StreamError::IdleTimeout(dur))).await;
                    }
                    break;
                }
            };
            match item {
                Ok(bytes) => {
                    // Convert bytes to string and process line by line
                    let text = match String::from_utf8(bytes.to_vec()) {
                        Ok(t) => t,
                        Err(e) => {
                            let _ = tx
                                .send(Err(StreamError::InvalidSse(format!(
                                    "Invalid UTF-8: {}",
                                    e
                                ))))
                                .await;
                            break;
                        }
                    };

                    buffer.push_str(&text);

                    // Process complete lines
                    while let Some(pos) = buffer.find('\n') {
                        let line: String = buffer.drain(..=pos).collect();

                        match parse_sse_line(&line) {
                            Ok(Some(content)) => {
                                let mut chunk = StreamChunk::delta(content);
                                if count_tokens {
                                    chunk = chunk.with_token_estimate();
                                }
                                if tx.send(Ok(chunk)).await.is_err() {
                                    return; // Receiver dropped
                                }
                            }
                            Ok(None) => {}
                            Err(e) => {
                                let _ = tx.send(Err(e)).await;
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(StreamError::Http(e))).await;
                    break;
                }
            }
        }

        // Send final chunk
        let _ = tx.send(Ok(StreamChunk::final_chunk())).await;
    });

    // Convert channel receiver to stream
    stream::unfold(rx, |mut rx| async {
        rx.recv().await.map(|chunk| (chunk, rx))
    })
    .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn idle_read_times_out_on_a_stalled_stream() {
        // #4761: a stream that yields nothing (upstream established the
        // connection then went silent mid-generation) must resolve to
        // `IdleTimedOut` within the idle window instead of blocking forever.
        // Without the timeout guard this `.await` never returns and the turn is
        // orphaned with no terminal event.
        let mut stalled = stream::pending::<u8>();
        let outcome = read_next_or_idle(&mut stalled, Some(Duration::from_millis(20))).await;
        assert_eq!(outcome, IdleRead::IdleTimedOut);
    }

    #[tokio::test]
    async fn idle_read_returns_item_when_data_arrives() {
        let mut s = stream::iter(vec![7u8, 8u8]);
        assert_eq!(
            read_next_or_idle(&mut s, Some(Duration::from_secs(5))).await,
            IdleRead::Item(7u8)
        );
        assert_eq!(
            read_next_or_idle(&mut s, Some(Duration::from_secs(5))).await,
            IdleRead::Item(8u8)
        );
    }

    #[tokio::test]
    async fn idle_read_reports_clean_end() {
        let mut empty = stream::iter(Vec::<u8>::new());
        assert_eq!(
            read_next_or_idle(&mut empty, Some(Duration::from_secs(5))).await,
            IdleRead::Ended
        );
    }

    #[tokio::test]
    async fn idle_read_with_guard_disabled_still_yields_items() {
        // `None` restores the pre-#4761 unbounded wait; a ready item must still
        // be delivered (the disabled guard only removes the timeout, not data).
        let mut s = stream::iter(vec![9u8]);
        assert_eq!(read_next_or_idle(&mut s, None).await, IdleRead::Item(9u8));
        assert_eq!(read_next_or_idle(&mut s, None).await, IdleRead::Ended);
    }

    #[test]
    fn idle_timeout_env_override_and_disable() {
        // Default (var unset) is the built-in guard; `0` disables it; a positive
        // value overrides it. Serialized within one test to avoid racing the
        // process-global env var across the test binary.
        let key = "OPENHUMAN_INFERENCE_STREAM_IDLE_TIMEOUT_SECS";
        let prev = std::env::var(key).ok();

        std::env::remove_var(key);
        assert_eq!(
            stream_idle_timeout(),
            Some(Duration::from_secs(DEFAULT_STREAM_IDLE_TIMEOUT_SECS))
        );

        std::env::set_var(key, "0");
        assert_eq!(stream_idle_timeout(), None, "0 disables the guard");

        std::env::set_var(key, "45");
        assert_eq!(stream_idle_timeout(), Some(Duration::from_secs(45)));

        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
    }
}
