//! Tool: ax_interact — interact with desktop app UI via the macOS Accessibility API.
//!
//! Uses AXUIElement (not CGEvent/enigo) so it:
//!   - Never crashes CEF (no synthetic key/mouse events injected system-wide)
//!   - Works regardless of which app is focused
//!   - Finds elements by semantic label, not pixel coordinates
//!
//! Three actions:
//!   list       — enumerate interactive elements in a running app
//!   press      — activate a button/control by label
//!   set_value  — type text into a field by label
//!
//! Requires: macOS Accessibility permission granted to OpenHuman.

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
        "Interact with a desktop application's UI using the macOS Accessibility API (AXUIElement). \
         Finds buttons, text fields, and controls by their label — no screen coordinates needed. \
         Actions: \
         'list' → show all interactive elements in the app; \
         'press' → click/activate a button or control by label (e.g. 'Play', 'Send', 'OK'); \
         'set_value' → type text into a field by label. \
         Always call 'list' first if you're not sure what elements exist. \
         Requires macOS Accessibility permission for OpenHuman."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["list", "press", "set_value"],
                    "description": "'list' = show interactive elements; 'press' = click a button/control; 'set_value' = type into a text field."
                },
                "app_name": {
                    "type": "string",
                    "description": "Display name of the running application (e.g. 'Music', 'Safari', 'Telegram')."
                },
                "label": {
                    "type": "string",
                    "description": "Partial label of the element to target (case-insensitive). For 'set_value', omit to target the first available text field."
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

        log::info!(
            "[ax_interact] ▶ action={action:?} app={app_name:?} label={label:?}"
        );

        if app_name.is_empty() {
            return Ok(ToolResult::error("app_name is required"));
        }

        let result = match action.as_str() {
            "list" => {
                match ax::ax_list_elements(&app_name) {
                    Ok(elements) if elements.is_empty() => {
                        log::info!("[ax_interact] list: no interactive elements found in '{app_name}'");
                        ToolResult::success(format!(
                            "No interactive elements found in '{app_name}'. \
                             The app may not expose its UI tree via Accessibility API, \
                             or OpenHuman may need Accessibility permission."
                        ))
                    }
                    Ok(elements) => {
                        log::info!(
                            "[ax_interact] list: found {} elements in '{app_name}'",
                            elements.len()
                        );
                        let lines: Vec<String> = elements
                            .iter()
                            .map(|e| format!("  [{role}] {label}", role = e.role, label = e.label))
                            .collect();
                        ToolResult::success(format!(
                            "Interactive elements in '{app_name}' ({} found):\n{}",
                            elements.len(),
                            lines.join("\n")
                        ))
                    }
                    Err(e) => {
                        log::warn!("[ax_interact] list failed: {e}");
                        ToolResult::error(e)
                    }
                }
            }

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
                    return Ok(ToolResult::error("'value' is required for action='set_value'"));
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
