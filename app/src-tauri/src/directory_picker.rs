//! Native directory chooser for memory-source configuration (#5831).
//!
//! The folder memory-source used to be picked with an
//! `<input type="file" webkitdirectory>` in the renderer. That element does
//! not expose a filesystem path: Chromium only carries one on the
//! non-standard `File.path` attribute, and only when the renderer has
//! filesystem-aware integration. Outside that renderer the handler fell back
//! to `webkitRelativePath.split('/')[0]` — the chosen directory's **name**
//! with its location discarded — and stored it. The source then looked
//! configured and could never sync, failing once per cycle forever with
//! `folder does not exist: docs`.
//!
//! A native dialog has no such gap: it returns an absolute path in every
//! renderer, on every platform.
//!
//! ## Why `rfd` and not `tauri-plugin-dialog`
//!
//! This mirrors [`crate::artifact_commands::save_artifact_via_dialog`]
//! (#3162), which is already in this shell and already talks to the OS
//! dialog APIs through `rfd`. Reusing it means **no new dependency, no new
//! plugin, and no capability-allowlist entry** — the crate is declared in
//! `Cargo.toml` with `default-features = false` plus `xdg-portal`, which is
//! what keeps Linux off GTK. Adding the dialog plugin for one command would
//! widen the dependency graph for a capability the shell already has.
//!
//! ## Trust boundary
//!
//! Deliberately none. Unlike the artifact commands — which re-validate that
//! a renderer-supplied path sits inside the artifacts tree, because there
//! the renderer *supplies* the path — this command takes no input and
//! returns only what the user chose in an OS-owned dialog. The renderer
//! cannot steer it at a directory, and picking a folder to index is the
//! user's decision to make anywhere on their disk.

/// Open the OS-native directory chooser and return the absolute path of the
/// directory the user selected.
///
/// Returns:
/// - `Ok(Some(path))` — the absolute path chosen.
/// - `Ok(None)` — the user dismissed the dialog. Not an error; the caller
///   leaves the field untouched.
/// - `Err(_)` — the dialog could not run (for example, no xdg-desktop
///   portal on a headless Linux host), or it somehow yielded a relative
///   path. The caller surfaces this rather than storing anything.
#[tauri::command]
pub async fn pick_directory_via_dialog() -> Result<Option<String>, String> {
    // On Linux this is the xdg-desktop portal (no GTK link); on macOS and
    // Windows the system panel. The await resolves when the user picks or
    // cancels.
    let handle = rfd::AsyncFileDialog::new().pick_folder().await;

    let Some(dir) = handle else {
        log::info!("[directory_picker] pick_directory_via_dialog cancelled by user");
        return Ok(None);
    };

    let path = dir.path().to_path_buf();

    // The OS dialogs all hand back absolute paths, so this is a belt-and-braces
    // check rather than a branch we expect to take. It exists because a
    // relative value reaching the store is the entire defect this command was
    // written to remove: an error here is recoverable and visible, whereas a
    // stored relative path is neither.
    if !path.is_absolute() {
        return Err(format!(
            "the directory chooser returned a non-absolute path: {}",
            path.display()
        ));
    }

    log::info!(
        "[directory_picker] pick_directory_via_dialog chose {}",
        path.display()
    );
    Ok(Some(path.display().to_string()))
}
