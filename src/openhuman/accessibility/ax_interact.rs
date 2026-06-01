//! AXUIElement interaction helpers — list, press, and set-value for named apps.
//!
//! Delegates to the unified Swift helper (`helper.rs`) which walks the AX tree
//! without injecting synthetic events (unlike enigo/CGEventPost). Works even
//! when OpenHuman is not the focused application, and never crashes CEF.
//!
//! macOS only. Non-macOS builds return `Err("ax_interact is macOS-only")`.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct AXElement {
    pub role: String,
    pub label: String,
}

/// List interactive UI elements (buttons, text fields, checkboxes, …) in `app_name`.
pub fn ax_list_elements(app_name: &str) -> Result<Vec<AXElement>, String> {
    #[cfg(target_os = "macos")]
    {
        let req = serde_json::json!({ "type": "ax_list", "app_name": app_name });
        let resp = super::helper::helper_send_receive(&req)?;
        if resp.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(err.to_string());
        }
        let elements: Vec<AXElement> = resp
            .get("elements")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        return Ok(elements);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = app_name;
        Err("ax_interact is macOS-only".into())
    }
}

/// Press the first UI element in `app_name` whose label contains `label` (case-insensitive).
pub fn ax_press_element(app_name: &str, label: &str) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let req = serde_json::json!({
            "type": "ax_press",
            "app_name": app_name,
            "label": label,
        });
        let resp = super::helper::helper_send_receive(&req)?;
        if resp.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(err.to_string());
        }
        let pressed = resp
            .get("pressed")
            .and_then(|v| v.as_str())
            .unwrap_or(label)
            .to_string();
        return Ok(format!("Pressed '{pressed}' in '{app_name}'."));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app_name, label);
        Err("ax_interact is macOS-only".into())
    }
}

/// Set the value of the first text field in `app_name` whose label contains `label`.
/// Pass an empty `label` to target the first available text field.
pub fn ax_set_field_value(app_name: &str, label: &str, value: &str) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let req = serde_json::json!({
            "type": "ax_set_value",
            "app_name": app_name,
            "label": label,
            "value": value,
        });
        let resp = super::helper::helper_send_receive(&req)?;
        if resp.get("ok").and_then(|v| v.as_bool()) == Some(false) {
            let err = resp
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            return Err(err.to_string());
        }
        let field = resp
            .get("field")
            .and_then(|v| v.as_str())
            .unwrap_or(label)
            .to_string();
        return Ok(format!("Set '{field}' in '{app_name}' to the provided value."));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app_name, label, value);
        Err("ax_interact is macOS-only".into())
    }
}
