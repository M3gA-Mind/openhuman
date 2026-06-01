//! AXUIElement interaction helpers — list, press, and set-value for named apps.
//!
//! Delegates to the unified Swift helper (`helper.rs`) which walks the AX tree
//! without injecting synthetic events (unlike enigo/CGEventPost). Works even
//! when OpenHuman is not the focused application, and never crashes CEF.
//!
//! macOS only. Non-macOS builds return `Err("ax_interact is macOS-only")`.

use serde::Deserialize;

#[cfg(test)]
#[path = "ax_interact_tests.rs"]
mod tests;

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
        return Ok(format!(
            "Set '{field}' in '{app_name}' to the provided value."
        ));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app_name, label, value);
        Err("ax_interact is macOS-only".into())
    }
}

/// High-level one-shot: search Apple Music for `query` and play the top result.
///
/// Encapsulates the full proven sequence in Rust so the agent doesn't have to
/// orchestrate it (and doesn't need shell access):
///   1. `open music://music.apple.com/search?term=<query>` to surface results
///   2. wait for results, then AX-find the matching song cell and press it
///      (this NAVIGATES into the song's detail page — it does not play yet)
///   3. wait, then press the Play button on the detail page (actually plays)
///   4. verify playback via AppleScript player state
///
/// `query` is the song + artist, e.g. "Highway to Hell AC/DC".
pub fn play_apple_music(query: &str) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::thread::sleep;
        use std::time::Duration;

        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err("query must not be empty".into());
        }
        log::info!("[play_apple_music] ▶ query={trimmed:?}");

        // 1. Open Music + search via URL scheme.
        let term = trimmed.replace(' ', "+");
        let url = format!("music://music.apple.com/search?term={term}");
        Command::new("open")
            .arg(&url)
            .status()
            .map_err(|e| format!("failed to open Music search URL: {e}"))?;
        log::info!("[play_apple_music] opened search url={url}");
        sleep(Duration::from_secs(4));

        // 2. Find the best-matching song cell and press to navigate in.
        //    Match on the leading song-title token (before the artist words).
        let elements = ax_list_elements("Music")?;
        log::info!("[play_apple_music] {} elements after search", elements.len());

        // Pick the first AXCell whose label looks like the requested song.
        // Try progressively shorter prefixes of the query for a label match.
        let lower_query = trimmed.to_lowercase();
        let candidate = elements
            .iter()
            .find(|e| {
                e.role == "AXCell" && {
                    let l = e.label.to_lowercase();
                    // song cell labels are just the title (e.g. "Highway to Hell")
                    !e.label.is_empty() && lower_query.contains(&l)
                }
            })
            .or_else(|| {
                // fallback: any cell whose label is contained in the query
                elements.iter().find(|e| {
                    e.role == "AXCell"
                        && !e.label.is_empty()
                        && e.label.split_whitespace().count() >= 1
                        && lower_query.contains(&e.label.to_lowercase())
                })
            });

        let song_label = match candidate {
            Some(c) => c.label.clone(),
            None => {
                let avail: Vec<String> = elements
                    .iter()
                    .filter(|e| e.role == "AXCell" && !e.label.is_empty())
                    .map(|e| e.label.clone())
                    .take(20)
                    .collect();
                return Err(format!(
                    "No matching song found for '{trimmed}'. Top result cells: {}",
                    avail.join(", ")
                ));
            }
        };

        log::info!("[play_apple_music] navigating into song cell: {song_label:?}");
        ax_press_element("Music", &song_label)?;
        sleep(Duration::from_secs(2));

        // 3. Press the Play button on the detail page.
        log::info!("[play_apple_music] pressing detail-page Play");
        ax_press_element("Music", "Play")?;
        sleep(Duration::from_secs(2));

        // 4. Verify playback.
        let state = Command::new("osascript")
            .args(["-e", "tell application \"Music\" to get player state"])
            .output()
            .map_err(|e| format!("failed to query player state: {e}"))?;
        let state_str = String::from_utf8_lossy(&state.stdout).trim().to_string();
        log::info!("[play_apple_music] player state={state_str}");

        if state_str.contains("playing") {
            let track = Command::new("osascript")
                .args(["-e", "tell application \"Music\" to get name of current track"])
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| song_label.clone());
            Ok(format!("Now playing '{track}' in Apple Music."))
        } else {
            Err(format!(
                "Pressed play for '{song_label}' but player state is '{state_str}'. \
                 The song may require a subscription or be unavailable."
            ))
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = query;
        Err("play_apple_music is macOS-only".into())
    }
}
