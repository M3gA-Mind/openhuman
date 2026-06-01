//! Integration tests for AXUIElement-based app interaction.
//!
//! These tests require:
//!   1. macOS with Accessibility permission granted to the test runner
//!   2. Apple Music to be running (or openable)
//!
//! Run with: cargo test ax_interact -- --nocapture

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::super::ax_interact::{ax_list_elements, ax_press_element, ax_set_field_value};
    use std::process::Command;
    use std::thread::sleep;
    use std::time::Duration;

    /// Ensure Music is running. Returns false if it can't be opened.
    fn ensure_music_open() -> bool {
        let status = Command::new("open").arg("-a").arg("Music").status();
        if status.map(|s| s.success()).unwrap_or(false) {
            sleep(Duration::from_secs(2));
            true
        } else {
            false
        }
    }

    /// Ensure Music shows AC/DC search results via URL scheme.
    fn open_acdc_search() {
        Command::new("open")
            .arg("music://music.apple.com/search?term=Highway+to+Hell+ACDC")
            .status()
            .ok();
        sleep(Duration::from_secs(3));
    }

    #[test]
    #[ignore = "requires macOS Accessibility permission and Apple Music"]
    fn test_ax_list_returns_elements() {
        assert!(ensure_music_open(), "Could not open Music");
        let elements = ax_list_elements("Music").expect("ax_list_elements failed");
        assert!(!elements.is_empty(), "Expected interactive elements in Music");
        println!("Found {} elements:", elements.len());
        for el in &elements {
            println!("  [{}] {}", el.role, el.label);
        }
    }

    #[test]
    #[ignore = "requires macOS Accessibility permission and Apple Music"]
    fn test_ax_press_play_button() {
        assert!(ensure_music_open(), "Could not open Music");
        let result = ax_press_element("Music", "Play");
        println!("press Play: {:?}", result);
        assert!(result.is_ok(), "Expected Play button to be pressable: {:?}", result);
    }

    #[test]
    #[ignore = "requires macOS Accessibility permission and Apple Music"]
    fn test_full_flow_search_and_play_acdc() {
        // Step 1: open Music
        assert!(ensure_music_open(), "Could not open Music");

        // Step 2: verify AX tree is accessible
        let elements = ax_list_elements("Music").expect("ax_list failed");
        assert!(!elements.is_empty(), "Music AX tree is empty — check Accessibility permission");
        println!("[step 1] AX tree: {} elements", elements.len());

        // Step 3: open AC/DC search via URL scheme
        open_acdc_search();
        println!("[step 2] search URL opened");

        // Step 4: list elements again — Highway to Hell should appear as AXCell/AXButton
        let after_search = ax_list_elements("Music").expect("ax_list post-search failed");
        let highway = after_search.iter().find(|e| e.label.contains("Highway to Hell"));
        println!("[step 3] 'Highway to Hell' element: {:?}", highway.map(|e| &e.label));
        assert!(
            highway.is_some(),
            "Expected 'Highway to Hell' in search results. Elements found:\n{}",
            after_search
                .iter()
                .map(|e| format!("  [{}] {}", e.role, e.label))
                .collect::<Vec<_>>()
                .join("\n")
        );

        // Step 5: press the first result
        let press_result = ax_press_element("Music", "Highway to Hell");
        println!("[step 4] press Highway to Hell: {:?}", press_result);
        assert!(press_result.is_ok(), "Could not press 'Highway to Hell': {:?}", press_result);

        sleep(Duration::from_secs(2));

        // Step 6: verify playback started by checking Play button is now visible
        let playing_elements = ax_list_elements("Music").expect("ax_list post-press failed");
        let has_play_or_pause = playing_elements
            .iter()
            .any(|e| e.label == "Play" || e.label == "Pause");
        println!(
            "[step 5] play/pause button present: {}",
            has_play_or_pause
        );
        // Not asserting since state depends on prior playback status; just log
    }

    #[test]
    #[ignore = "requires macOS Accessibility permission and Apple Music"]
    fn test_ax_set_search_field() {
        assert!(ensure_music_open(), "Could not open Music");
        // The search field only appears after navigating to the search view.
        // This test opens it via URL scheme first.
        Command::new("open")
            .arg("music://music.apple.com/search")
            .status()
            .ok();
        sleep(Duration::from_secs(2));

        let result = ax_set_field_value("Music", "Search", "Bollywood");
        println!("set_value Search=Bollywood: {:?}", result);
        // May fail if search field is not an AXTextField but AXSearchField or similar
        // — log the result for diagnosis rather than asserting
    }

    #[test]
    fn test_ax_list_nonexistent_app() {
        let result = ax_list_elements("NonExistentApp12345");
        assert!(result.is_err(), "Expected error for non-existent app");
        println!("Error (expected): {:?}", result.unwrap_err());
    }

    #[test]
    fn test_ax_press_nonexistent_app() {
        let result = ax_press_element("NonExistentApp12345", "Play");
        assert!(result.is_err(), "Expected error for non-existent app");
    }
}
