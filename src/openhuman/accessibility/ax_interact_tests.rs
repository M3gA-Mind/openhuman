//! Integration tests for AXUIElement-based app interaction.
//!
//! These tests require:
//!   1. macOS with Accessibility permission granted to the test runner
//!   2. Apple Music to be running (or openable)
//!
//! Run with: cargo test ax_interact -- --nocapture --include-ignored

#![cfg(all(test, target_os = "macos"))]

use super::{ax_list_elements, ax_press_element, ax_set_field_value};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

fn ensure_music_open() -> bool {
    let ok = Command::new("open")
        .arg("-a")
        .arg("Music")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        sleep(Duration::from_secs(2));
    }
    ok
}

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
    assert!(ensure_music_open(), "Could not open Music");

    let elements = ax_list_elements("Music").expect("ax_list failed");
    assert!(!elements.is_empty(), "Music AX tree empty — check Accessibility permission");
    println!("[step 1] AX tree: {} elements", elements.len());

    open_acdc_search();
    println!("[step 2] search URL opened");

    let after_search = ax_list_elements("Music").expect("ax_list post-search failed");
    let highway = after_search.iter().find(|e| e.label.contains("Highway to Hell"));
    println!("[step 3] 'Highway to Hell' element: {:?}", highway.map(|e| &e.label));
    assert!(
        highway.is_some(),
        "Expected 'Highway to Hell' in results. Found:\n{}",
        after_search.iter().map(|e| format!("  [{}] {}", e.role, e.label)).collect::<Vec<_>>().join("\n")
    );

    let press_result = ax_press_element("Music", "Highway to Hell");
    println!("[step 4] press: {:?}", press_result);
    assert!(press_result.is_ok(), "Could not press song: {:?}", press_result);

    sleep(Duration::from_secs(2));

    let playing = ax_list_elements("Music").expect("ax_list post-press failed");
    let has_play_pause = playing.iter().any(|e| e.label == "Play" || e.label == "Pause");
    println!("[step 5] play/pause present: {has_play_pause}");
}

#[test]
#[ignore = "requires macOS Accessibility permission and Apple Music"]
fn test_ax_set_search_field() {
    assert!(ensure_music_open(), "Could not open Music");
    Command::new("open").arg("music://music.apple.com/search").status().ok();
    sleep(Duration::from_secs(2));
    let result = ax_set_field_value("Music", "Search", "Bollywood");
    println!("set_value Search=Bollywood: {:?}", result);
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
    assert!(result.is_err());
}
