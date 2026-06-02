//! Tool: ax_interact — interact with desktop app UI via the OS accessibility API.
//!
//! Cross-platform: macOS uses AXUIElement (Swift helper), Windows uses UI
//! Automation (UIA COM API). Both back-ends:
//!   - Never crash CEF (no synthetic key/mouse events injected system-wide)
//!   - Work regardless of which app is focused
//!   - Find elements by semantic label, not pixel coordinates
//!
//! Three actions:
//!   list       — enumerate interactive elements in a running app
//!   press      — activate a button/control by label
//!   set_value  — type text into a field by label
//!
//! Requires: macOS Accessibility permission granted to OpenHuman. On Windows no
//! special permission is needed for same-integrity-level apps (UIPI blocks
//! driving an elevated app from a non-elevated process).

use crate::openhuman::accessibility::ax_interact as ax;
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolCallOptions, ToolResult};
use async_trait::async_trait;
use serde_json::json;

pub struct AxInteractTool;

impl AxInteractTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AxInteractTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AxInteractTool {
    fn name(&self) -> &str {
        "ax_interact"
    }

    fn description(&self) -> &str {
        "Interact with ANY desktop application's UI using the platform accessibility API \
         (macOS AXUIElement / Windows UI Automation). Finds buttons, text fields, list rows, \
         and controls by their label — no screen coordinates, no synthetic key/mouse events. \
         Works for any app: a music player, browser, mail, notes, Slack, system settings, etc.\n\
         \n\
         Actions:\n\
         • 'list' → show interactive elements. ALWAYS pass a `filter` substring to narrow \
         results (apps expose hundreds of elements; an unfiltered list is huge and unreliable). \
         e.g. filter='Play', filter='Send', filter='Highway'.\n\
         • 'press' → activate a button/control/row by label (exact match preferred). \
         e.g. label='Play', label='Send', label='OK'.\n\
         • 'set_value' → type text into a field by label (omit label for the first text field).\n\
         \n\
         General pattern: (1) `list` with a `filter` to find the element, (2) `press` it. \
         Note that in many apps, pressing a LIST ROW or SEARCH RESULT only selects/opens it — \
         to trigger an action you then press the relevant action button (e.g. after opening a \
         song's page, press its 'Play' button). If a press doesn't have the intended effect, \
         `list` again to see the new screen and press the actual action control.\n\
         \n\
         On macOS this requires Accessibility permission for OpenHuman; on Windows no special \
         permission is needed for normal (non-elevated) apps."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "press", "set_value"],
                    "description": "'list' = show interactive elements (use with filter); 'press' = activate a control by label; 'set_value' = type into a text field."
                },
                "app_name": {
                    "type": "string",
                    "description": "Display name of the running application (e.g. 'Music', 'Safari', 'Telegram')."
                },
                "filter": {
                    "type": "string",
                    "description": "For 'list': only return elements whose label contains this substring (case-insensitive). Strongly recommended — keeps results small and accurate."
                },
                "label": {
                    "type": "string",
                    "description": "For 'press'/'set_value': label of the element to target (case-insensitive, exact match preferred). For 'set_value', omit to target the first text field."
                },
                "value": {
                    "type": "string",
                    "description": "Text to enter (required for 'set_value')."
                }
            },
            "required": ["action", "app_name"]
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        // AXUIElement actions are semantic and targeted — much safer than
        // raw CGEventPost. ReadOnly permission means no approval gate fires,
        // keeping the voice-command flow smooth.
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
        let action = args
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let app_name = args
            .get("app_name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let label = args
            .get("label")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let value = args
            .get("value")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let filter = args
            .get("filter")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();

        log::info!(
            "[ax_interact] ▶ action={action:?} app={app_name:?} label={label:?} filter={filter:?}"
        );

        if app_name.is_empty() {
            return Ok(ToolResult::error("app_name is required"));
        }

        // Cap how many elements we render so a broad/empty filter can't overflow
        // the tool-result budget and cause the model to reason over a truncated view.
        const MAX_LISTED: usize = 60;

        let result = match action.as_str() {
            "list" => match ax::ax_list_elements_filtered(&app_name, &filter) {
                Ok(elements) if elements.is_empty() => {
                    log::info!(
                        "[ax_interact] list: no elements in '{app_name}' (filter={filter:?})"
                    );
                    let hint = if filter.is_empty() {
                        format!(
                            "No interactive elements found in '{app_name}'. \
                             The app may not expose its UI tree via Accessibility API, \
                             or OpenHuman may need Accessibility permission."
                        )
                    } else {
                        format!(
                            "No elements in '{app_name}' match filter '{filter}'. \
                             The UI may still be loading — wait and try again, or call \
                             'list' with a shorter/different filter."
                        )
                    };
                    ToolResult::success(hint)
                }
                Ok(elements) => {
                    let total = elements.len();
                    log::info!(
                        "[ax_interact] list: {total} elements in '{app_name}' (filter={filter:?})"
                    );
                    let shown = total.min(MAX_LISTED);
                    let lines: Vec<String> = elements
                        .iter()
                        .take(MAX_LISTED)
                        .map(|e| format!("  [{role}] {label}", role = e.role, label = e.label))
                        .collect();
                    let mut out = if filter.is_empty() {
                        format!("Elements in '{app_name}' (showing {shown} of {total}):\n")
                    } else {
                        format!(
                            "Elements in '{app_name}' matching '{filter}' (showing {shown} of {total}):\n"
                        )
                    };
                    out.push_str(&lines.join("\n"));
                    if total > MAX_LISTED {
                        out.push_str(&format!(
                            "\n… {} more — narrow with a more specific `filter`.",
                            total - MAX_LISTED
                        ));
                    }
                    ToolResult::success(out)
                }
                Err(e) => {
                    log::warn!("[ax_interact] list failed: {e}");
                    ToolResult::error(e)
                }
            },

            "press" => {
                if label.is_empty() {
                    return Ok(ToolResult::error(
                        "'label' is required for action='press'. Use action='list' first to discover element labels.",
                    ));
                }
                match ax::ax_press_element(&app_name, &label) {
                    Ok(msg) => {
                        log::info!("[ax_interact] press succeeded: {msg}");
                        ToolResult::success(msg)
                    }
                    Err(e) => {
                        log::warn!("[ax_interact] press failed: {e}");
                        ToolResult::error(format!(
                            "{e}. Try action='list' to see available element labels."
                        ))
                    }
                }
            }

            "set_value" => {
                if value.is_empty() {
                    return Ok(ToolResult::error(
                        "'value' is required for action='set_value'",
                    ));
                }
                match ax::ax_set_field_value(&app_name, &label, &value) {
                    Ok(msg) => {
                        log::info!("[ax_interact] set_value succeeded: {msg}");
                        ToolResult::success(msg)
                    }
                    Err(e) => {
                        log::warn!("[ax_interact] set_value failed: {e}");
                        ToolResult::error(format!(
                            "{e}. Try action='list' to see available text field labels."
                        ))
                    }
                }
            }

            other => ToolResult::error(format!(
                "Unknown action '{other}'. Valid actions: 'list', 'press', 'set_value'."
            )),
        };

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_and_permission() {
        let tool = AxInteractTool::new();
        assert_eq!(tool.name(), "ax_interact");
        assert_eq!(tool.permission_level(), PermissionLevel::ReadOnly);
    }

    #[test]
    fn schema_requires_action_and_app_name() {
        let schema = AxInteractTool::new().parameters_schema();
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v == "action"));
        assert!(required.iter().any(|v| v == "app_name"));
    }

    #[tokio::test]
    async fn rejects_missing_app_name() {
        let result = AxInteractTool::new()
            .execute(json!({"action": "list", "app_name": ""}))
            .await
            .unwrap();
        assert!(result.is_error);
    }

    #[tokio::test]
    async fn rejects_press_without_label() {
        let result = AxInteractTool::new()
            .execute(json!({"action": "press", "app_name": "Music"}))
            .await
            .unwrap();
        assert!(result.is_error);
    }
}
