//! Pure failure classification: raw tool error text → [`ClassifiedFailure`].
//!
//! This is deliberately a heuristic, keyword-driven mapping over the error
//! string the executor already produces (`agent_tool_exec` collapses a tool
//! outcome to a message + `success` flag). It touches no global state and does
//! no I/O, so every branch is unit-testable and stays cheap to call on the hot
//! tool-execution path.
//!
//! Precedence matters: the first matching class wins, so the checks are ordered
//! most-specific → least-specific. Anything unmatched falls through to
//! [`ToolFailureClass::Unknown`] (treated as recoverable so a later retry phase
//! can give it one bounded attempt rather than surfacing a dead end).

use super::types::{ClassifiedFailure, FailureCategory, ToolFailureClass};

/// Classify a failed tool call into a user-facing [`ClassifiedFailure`].
///
/// * `error_text` — the raw error/output message from the tool. Matched
///   case-insensitively; never surfaced verbatim to the user.
/// * `timed_out` — set by the executor when the failure was a deadline stop
///   (the message alone is not always reliable), which short-circuits to
///   [`ToolFailureClass::Timeout`].
pub fn classify(error_text: &str, timed_out: bool) -> ClassifiedFailure {
    let class = classify_class(error_text, timed_out);
    describe(class)
}

/// The class-only half of [`classify`], split out so the ordering heuristics can
/// be tested independently of the copy.
fn classify_class(error_text: &str, timed_out: bool) -> ToolFailureClass {
    let text = error_text.to_lowercase();

    // 1. Timeout — the executor's explicit signal wins over any text sniffing.
    if timed_out || contains_any(&text, &["timed out", "timeout", "deadline exceeded"]) {
        return ToolFailureClass::Timeout;
    }

    // 2. Bad credentials — 401 / auth-token problems (distinct from 403 policy).
    if contains_any(
        &text,
        &[
            "unauthorized",
            "401",
            "invalid api key",
            "invalid_api_key",
            "authentication failed",
            "invalid credentials",
            "bad credentials",
            "token expired",
            "invalid_grant",
            "not signed in",
            "sign in again",
        ],
    ) {
        return ToolFailureClass::BadCredentials;
    }

    // 3. Blocked by policy — the security/autonomy gate or a forbidden path.
    //    `channel allows` is the tail of the tool-policy PermissionDenied render.
    if contains_any(
        &text,
        &[
            "blocked by policy",
            "security policy",
            "channel allows",
            "not allowed by",
            "forbidden path",
            "403",
            "forbidden",
            "autonomy",
            "policy denied",
        ],
    ) {
        return ToolFailureClass::BlockedByPolicy;
    }

    // 4. Missing OS/tool permission — access denied at the filesystem/OS layer.
    if contains_any(
        &text,
        &[
            "permission denied",
            "os error 13",
            "eacces",
            "operation not permitted",
            "access is denied",
            "not permitted",
        ],
    ) {
        return ToolFailureClass::MissingPermission;
    }

    // 5. Missing app/command — the thing we tried to invoke isn't there.
    if contains_any(
        &text,
        &[
            "command not found",
            "not installed",
            "no such application",
            "could not find application",
            "executable not found",
            "is not recognized as",
            "no such file or directory (os error 2)",
        ],
    ) {
        return ToolFailureClass::MissingApp;
    }

    // 6. Model / provider connectivity — before generic service errors so a
    //    provider outage is named specifically.
    if contains_any(
        &text,
        &[
            "provider error",
            "could not reach the model",
            "could not reach model",
            "ollama",
            "llm provider",
            "inference failed",
            "model endpoint",
            "no route to host",
        ],
    ) {
        return ToolFailureClass::ModelConnection;
    }

    // 7. Service unavailable — generic transient upstream/network failure.
    if contains_any(
        &text,
        &[
            "connection refused",
            "econnrefused",
            "service unavailable",
            "503",
            "502",
            "504",
            "temporarily unavailable",
            "could not connect",
            "connection reset",
            "network is unreachable",
        ],
    ) {
        return ToolFailureClass::ServiceUnavailable;
    }

    ToolFailureClass::Unknown
}

/// Attach the category + plain-language copy for a known class. Use this at the
/// call sites that already know the class for certain (e.g. the policy gate,
/// which knows a refusal is [`ToolFailureClass::BlockedByPolicy`]) rather than
/// round-tripping a synthetic message through [`classify`]. The copy is the
/// user-facing English source string; the UI localizes by class, so keep these
/// stable and jargon-free.
pub fn describe(class: ToolFailureClass) -> ClassifiedFailure {
    let category = class.category();
    let (cause_plain, next_action) = match class {
        ToolFailureClass::MissingPermission => (
            "OpenHuman doesn't have permission to do this yet.",
            "Grant the permission it needs, then try again.",
        ),
        ToolFailureClass::MissingApp => (
            "The app or program needed for this action isn't available.",
            "Install or open the app, then try again.",
        ),
        ToolFailureClass::ServiceUnavailable => (
            "A service OpenHuman needs is temporarily unavailable.",
            "OpenHuman will try again shortly — no action needed.",
        ),
        ToolFailureClass::BadCredentials => (
            "The saved sign-in details are missing or no longer valid.",
            "Sign in again or update the credentials, then try again.",
        ),
        ToolFailureClass::BlockedByPolicy => (
            "This action is blocked by your safety settings.",
            "Allow it in Settings → Agent access if you want it to run.",
        ),
        ToolFailureClass::ModelConnection => (
            "OpenHuman couldn't reach the AI model.",
            "Check your connection or model settings; OpenHuman will retry.",
        ),
        ToolFailureClass::Timeout => (
            "The action took too long and was stopped.",
            "OpenHuman will try again, or you can retry it manually.",
        ),
        ToolFailureClass::Unknown => (
            "Something went wrong with this action.",
            "Try again; if it keeps failing, run diagnostics from Settings.",
        ),
    };
    ClassifiedFailure {
        class,
        category,
        cause_plain: cause_plain.to_string(),
        next_action: next_action.to_string(),
        recoverable: category.is_recoverable(),
    }
}

/// Case-insensitive: does `haystack` (already lowercased) contain any needle?
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_of(text: &str) -> ToolFailureClass {
        classify(text, false).class
    }

    #[test]
    fn timeout_flag_wins_regardless_of_text() {
        assert_eq!(
            classify("anything at all", true).class,
            ToolFailureClass::Timeout
        );
    }

    #[test]
    fn timeout_detected_from_text() {
        assert_eq!(
            class_of("tool 'shell' timed out after 120 seconds"),
            ToolFailureClass::Timeout
        );
    }

    #[test]
    fn missing_permission_from_os_error() {
        assert_eq!(
            class_of("Error executing file_write: Permission denied (os error 13)"),
            ToolFailureClass::MissingPermission
        );
        assert_eq!(
            class_of("EACCES: operation not permitted"),
            ToolFailureClass::MissingPermission
        );
    }

    #[test]
    fn missing_app_from_command_not_found() {
        assert_eq!(
            class_of("bash: gh: command not found"),
            ToolFailureClass::MissingApp
        );
        assert_eq!(
            class_of("ffmpeg is not installed on this system"),
            ToolFailureClass::MissingApp
        );
    }

    #[test]
    fn service_unavailable_from_connection_errors() {
        assert_eq!(
            class_of("connection refused (ECONNREFUSED)"),
            ToolFailureClass::ServiceUnavailable
        );
        assert_eq!(
            class_of("upstream returned 503 Service Unavailable"),
            ToolFailureClass::ServiceUnavailable
        );
    }

    #[test]
    fn bad_credentials_from_auth_errors() {
        assert_eq!(
            class_of("HTTP 401 Unauthorized"),
            ToolFailureClass::BadCredentials
        );
        assert_eq!(
            class_of("invalid api key provided"),
            ToolFailureClass::BadCredentials
        );
        assert_eq!(
            class_of("auth token expired, please sign in again"),
            ToolFailureClass::BadCredentials
        );
    }

    #[test]
    fn blocked_by_policy_from_gate_and_forbidden() {
        assert_eq!(
            class_of(
                "Permission denied for tool 'shell': requires Execute, channel allows ReadOnly"
            ),
            ToolFailureClass::BlockedByPolicy
        );
        assert_eq!(
            class_of("blocked by policy: destructive command"),
            ToolFailureClass::BlockedByPolicy
        );
        assert_eq!(
            class_of("HTTP 403 Forbidden"),
            ToolFailureClass::BlockedByPolicy
        );
    }

    #[test]
    fn model_connection_from_provider_errors() {
        assert_eq!(
            class_of("Provider error (retryable=true): boom"),
            ToolFailureClass::ModelConnection
        );
        assert_eq!(
            class_of("could not reach the model endpoint"),
            ToolFailureClass::ModelConnection
        );
        assert_eq!(
            class_of("ollama daemon not responding"),
            ToolFailureClass::ModelConnection
        );
    }

    #[test]
    fn unknown_when_nothing_matches() {
        assert_eq!(
            class_of("some totally novel failure mode"),
            ToolFailureClass::Unknown
        );
    }

    #[test]
    fn credentials_precedence_over_service_when_both_present() {
        // A 401 that also mentions a connection should read as credentials, not
        // a transient service blip — the ordering guarantees this.
        assert_eq!(
            class_of("could not connect: 401 unauthorized"),
            ToolFailureClass::BadCredentials
        );
    }

    #[test]
    fn every_class_produces_nonempty_user_copy() {
        for class in [
            ToolFailureClass::MissingPermission,
            ToolFailureClass::MissingApp,
            ToolFailureClass::ServiceUnavailable,
            ToolFailureClass::BadCredentials,
            ToolFailureClass::BlockedByPolicy,
            ToolFailureClass::ModelConnection,
            ToolFailureClass::Timeout,
            ToolFailureClass::Unknown,
        ] {
            let f = describe(class);
            assert!(!f.cause_plain.is_empty(), "empty cause for {class:?}");
            assert!(!f.next_action.is_empty(), "empty next_action for {class:?}");
            assert_eq!(f.recoverable, f.category.is_recoverable());
        }
    }

    #[test]
    fn recoverable_flag_matches_category() {
        assert!(classify("503 service unavailable", false).recoverable);
        assert!(!classify("permission denied (os error 13)", false).recoverable);
        assert!(!classify("blocked by policy", false).recoverable);
    }
}
