//! Tool: play_music — search Apple Music for a song and play it, in one call.
//!
//! Encapsulates the full proven sequence (search URL → navigate into the song →
//! press the detail-page Play button → verify playback) in native Rust so the
//! agent does not have to orchestrate multiple `ax_interact` steps or have shell
//! access. Returns only after confirming `player state == playing`.
//!
//! macOS + Apple Music only.

use crate::openhuman::accessibility::ax_interact as ax;
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolCallOptions, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct PlayMusicTool;

impl PlayMusicTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PlayMusicTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for PlayMusicTool {
    fn name(&self) -> &str {
        "play_music"
    }

    fn description(&self) -> &str {
        "Play a specific song in Apple Music in ONE call. Pass the song and artist as \
         `query` (e.g. 'Highway to Hell AC/DC', 'Numb Linkin Park'). This opens Music, \
         searches, navigates into the song, presses Play, and confirms playback started. \
         Use this for any 'play <song>' request instead of ax_interact — it handles the \
         whole flow reliably. macOS + Apple Music only."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Song title and artist to search and play, e.g. 'Highway to Hell AC/DC'."
                }
            },
            "required": ["query"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        // Playing music is a benign, user-requested action — no approval gate.
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: serde_json::Value) -> anyhow::Result<ToolResult> {
        self.execute_with_options(args, ToolCallOptions::default())
            .await
    }

    async fn execute_with_options(
        &self,
        args: serde_json::Value,
        _options: ToolCallOptions,
    ) -> anyhow::Result<ToolResult> {
        let query = args
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        log::info!("[play_music] ▶ query={query:?}");

        if query.is_empty() {
            return Ok(ToolResult::error("query is required (song + artist)"));
        }

        // Run the blocking AX/AppleScript sequence off the async runtime thread.
        let result =
            tokio::task::spawn_blocking(move || ax::play_apple_music(&query))
                .await
                .map_err(|e| anyhow::anyhow!("play_music task panicked: {e}"))?;

        match result {
            Ok(msg) => {
                log::info!("[play_music] ✓ {msg}");
                Ok(ToolResult::success(msg))
            }
            Err(e) => {
                log::warn!("[play_music] ✗ {e}");
                Ok(ToolResult::error(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_permission() {
        let tool = PlayMusicTool::new();
        assert_eq!(tool.name(), "play_music");
        assert_eq!(tool.permission_level(), PermissionLevel::ReadOnly);
    }

    #[test]
    fn schema_requires_query() {
        let schema = PlayMusicTool::new().parameters_schema();
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "query"));
    }

    #[tokio::test]
    async fn rejects_empty_query() {
        let result = PlayMusicTool::new()
            .execute(json!({"query": ""}))
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
