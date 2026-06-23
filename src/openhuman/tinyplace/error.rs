//! Error classification for the tiny.place client-build chokepoint.
//!
//! Every `tinyplace_*` controller obtains its client via
//! [`TinyPlaceState::client`](super::state::TinyPlaceState::client), which
//! derives a signer seed from the user's wallet. When the user has not set up
//! a wallet yet, that derivation fails with the plain string
//! [`wallet::WALLET_NOT_CONFIGURED_MESSAGE`]. home_feed (backend
//! `GraphQLAuth::Agent`) legitimately needs a configured signer, so there is no
//! local lever to make the call succeed — this is an **expected user state**,
//! not an internal failure.
//!
//! Left as an unstructured string, the JSON-RPC boundary (`src/core/jsonrpc.rs`)
//! treats it as `expected_user_state = false` and escalates it to an
//! error-level Sentry capture — flooding Sentry with noise from wallet-less
//! users opening the Agent World Feed (issue #3964).
//!
//! [`classify_client_build_error`] reclassifies **only that exact message**
//! into a [`StructuredRpcError`] with `expected_user_state: true`, mirroring the
//! `threads::NotFound → ThreadNotFound` pattern. Every other failure
//! (decrypt, key derivation, config load) passes through unchanged and keeps
//! paging.

use serde_json::json;

use crate::openhuman::wallet::WALLET_NOT_CONFIGURED_MESSAGE;
use crate::rpc::StructuredRpcError;

/// Stable JSON-RPC discriminator the frontend can branch on to render the
/// "set up wallet" prompt without string-matching the message.
pub const WALLET_NOT_CONFIGURED_KIND: &str = "WalletNotConfigured";

/// Classify an error returned while building the tiny.place client.
///
/// The wallet-not-configured condition is reclassified into a structured,
/// expected-user-state envelope so the JSON-RPC boundary skips Sentry. Any
/// other error string is returned verbatim so genuine failures still page.
pub(crate) fn classify_client_build_error(err: String) -> String {
    if err == WALLET_NOT_CONFIGURED_MESSAGE {
        StructuredRpcError {
            message: err,
            data: Some(json!({ "kind": WALLET_NOT_CONFIGURED_KIND })),
            expected_user_state: true,
        }
        .encode()
    } else {
        err
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wallet_not_configured_becomes_expected_user_state_envelope() {
        let raw = classify_client_build_error(WALLET_NOT_CONFIGURED_MESSAGE.to_string());
        let structured = StructuredRpcError::decode(&raw)
            .expect("wallet-not-configured must emit a structured envelope");
        assert_eq!(structured.message, WALLET_NOT_CONFIGURED_MESSAGE);
        assert!(
            structured.expected_user_state,
            "wallet-not-configured is an expected user state, not a Sentry error"
        );
        let data = structured.data.expect("structured error must carry data");
        assert_eq!(data["kind"], WALLET_NOT_CONFIGURED_KIND);
    }

    #[test]
    fn unrelated_errors_pass_through_and_still_page() {
        for err in [
            "decrypt failed: bad key",
            "tinyplace signer init: invalid seed length",
            "config load timed out",
        ] {
            let raw = classify_client_build_error(err.to_string());
            assert_eq!(raw, err, "non-wallet errors must be returned verbatim");
            assert!(
                StructuredRpcError::decode(&raw).is_none(),
                "non-wallet errors must NOT carry the expected-user-state sentinel"
            );
        }
    }

    /// Coupling guard: the classifier keys off the *exact* string the wallet
    /// layer emits. This fails loudly if either side drifts, preventing a
    /// silent regression back to error-level Sentry noise (#3964).
    #[test]
    fn classifier_is_coupled_to_the_wallet_message_constant() {
        assert_eq!(
            WALLET_NOT_CONFIGURED_MESSAGE,
            "wallet is not configured; run wallet setup first",
            "the tiny.place classifier and the wallet origin must agree on this \
             exact string; update both sites together if it changes"
        );
    }
}
