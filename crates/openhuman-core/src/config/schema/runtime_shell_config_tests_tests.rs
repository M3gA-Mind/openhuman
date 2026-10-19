use super::ShellConfig;

#[test]
fn shell_config_defaults_hide_window_on() {
    // #7895: a default Windows install must not flash a console per shell call.
    assert!(ShellConfig::default().hide_window);
}

#[test]
fn shell_config_parses_hide_window_from_toml() {
    let cfg: ShellConfig = toml::from_str("hide_window = false").unwrap();
    assert!(
        !cfg.hide_window,
        "explicit opt-out must survive the true default"
    );
}

#[test]
fn shell_config_empty_table_keeps_default() {
    // An empty `[shell]` table relies on `#[serde(default)]` for the field.
    let cfg: ShellConfig = toml::from_str("").unwrap();
    assert!(cfg.hide_window);
}
