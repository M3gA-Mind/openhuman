//! Uniswap V3 swap helpers, scoped to a single curated pair on Base:
//! ETH → TOSHI via the 1% WETH/TOSHI pool (the deeper of the two pools).
//!
//! This module is intentionally narrow:
//! - It only knows about Base mainnet.
//! - It only encodes/decodes the two Uniswap V3 functions we need:
//!   `SwapRouter02.exactInputSingle` and `QuoterV2.quoteExactInputSingle`.
//! - It uses `msg.value` for the input (no explicit WETH wrap step) — the
//!   SwapRouter's `_pay` helper auto-wraps when `tokenIn == WETH9` and
//!   `address(this).balance >= value`.
//!
//! Anything wider (other pairs, other chains, other DEXes) belongs in a
//! separate module so this one stays auditable. Real ETH leaves the wallet
//! when `buy_toshi_on_base` lands — every change here is security-sensitive.

use std::str::FromStr;

use ethers_core::abi::{Function, Param, ParamType, StateMutability, Token};
use ethers_core::types::{Address, U256};
use log::debug;
use serde_json::json;

use crate::rpc::RpcOutcome;

use super::defaults::EvmNetwork;
use super::execution::{
    execute_prepared, hex_to_bytes, next_quote_id, require_account, store_quote,
    ExecutePreparedParams, ExecutionResult, PreparedKind, PreparedStatus, PreparedTransaction,
    QUOTE_TTL_MS,
};
use super::ops::WalletChain;
use super::rpc::evm_rpc_call;

const LOG_PREFIX: &str = "[wallet::uniswap]";

// ── Base mainnet addresses ────────────────────────────────────────────────
//
// WETH9 on Base — canonical wrapped-ETH. SwapRouter02 wraps msg.value into
// this when tokenIn == WETH9.
pub const BASE_WETH: &str = "0x4200000000000000000000000000000000000006";
// Toshi (TOSHI) — Base-native memecoin. Verified via geckoterminal / basescan.
pub const BASE_TOSHI: &str = "0xac1bd2486aaf3b5c0fc3fd868558b082a531b2b4";
// Uniswap V3 SwapRouter02 on Base.
pub const BASE_UNISWAP_V3_ROUTER: &str = "0x2626664c2603336E57B271c5C0b26F421741e481";
// Uniswap V3 QuoterV2 on Base.
pub const BASE_UNISWAP_V3_QUOTER: &str = "0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a";
// 1% pool fee tier — the WETH/TOSHI 1% pool has ~3x the liquidity of the
// 0.3% pool at the time of writing, so it gives better execution for our
// $10-ish demo swap.
pub const TOSHI_POOL_FEE_BPS: u32 = 10_000;
// Slippage tolerance (1.00%) for converting QuoterV2 amountOut → amountOutMinimum.
pub const DEFAULT_SLIPPAGE_BPS: u32 = 100;

/// Build calldata for `SwapRouter02.exactInputSingle((WETH, TOSHI, 10000,
/// recipient, amountIn, amountOutMin, 0))`. The router auto-wraps msg.value
/// when tokenIn == WETH, so the caller MUST also send `value == amountIn`.
pub fn encode_exact_input_single_eth_to_toshi(
    recipient: &str,
    amount_in_wei: U256,
    amount_out_min: U256,
) -> Result<String, String> {
    let weth = Address::from_str(BASE_WETH).expect("BASE_WETH constant must parse");
    let toshi = Address::from_str(BASE_TOSHI).expect("BASE_TOSHI constant must parse");
    let recipient_addr = Address::from_str(recipient.trim())
        .map_err(|e| format!("invalid recipient address '{recipient}': {e}"))?;

    let tuple = Token::Tuple(vec![
        Token::Address(weth),
        Token::Address(toshi),
        Token::Uint(U256::from(TOSHI_POOL_FEE_BPS)),
        Token::Address(recipient_addr),
        Token::Uint(amount_in_wei),
        Token::Uint(amount_out_min),
        // sqrtPriceLimitX96 = 0 disables the price limit (Uniswap convention).
        Token::Uint(U256::zero()),
    ]);

    #[allow(deprecated)]
    let function = Function {
        name: "exactInputSingle".to_string(),
        inputs: vec![Param {
            name: "params".to_string(),
            kind: exact_input_single_param_type(),
            internal_type: None,
        }],
        outputs: vec![Param {
            name: "amountOut".to_string(),
            kind: ParamType::Uint(256),
            internal_type: None,
        }],
        constant: None,
        state_mutability: StateMutability::Payable,
    };
    let bytes = function
        .encode_input(&[tuple])
        .map_err(|e| format!("failed to encode exactInputSingle calldata: {e}"))?;
    Ok(format!("0x{}", hex::encode(bytes)))
}

/// Build calldata for `QuoterV2.quoteExactInputSingle((WETH, TOSHI, amountIn,
/// 10000, 0))`. Note that QuoterV2 has a different param order than V1:
/// `(tokenIn, tokenOut, amountIn, fee, sqrtPriceLimitX96)`.
pub fn encode_quote_exact_input_single_eth_to_toshi(amount_in_wei: U256) -> Result<String, String> {
    let weth = Address::from_str(BASE_WETH).expect("BASE_WETH constant must parse");
    let toshi = Address::from_str(BASE_TOSHI).expect("BASE_TOSHI constant must parse");
    let tuple = Token::Tuple(vec![
        Token::Address(weth),
        Token::Address(toshi),
        Token::Uint(amount_in_wei),
        Token::Uint(U256::from(TOSHI_POOL_FEE_BPS)),
        Token::Uint(U256::zero()),
    ]);

    #[allow(deprecated)]
    let function = Function {
        name: "quoteExactInputSingle".to_string(),
        inputs: vec![Param {
            name: "params".to_string(),
            kind: quote_exact_input_single_param_type(),
            internal_type: None,
        }],
        outputs: vec![
            Param {
                name: "amountOut".to_string(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
            Param {
                name: "sqrtPriceX96After".to_string(),
                kind: ParamType::Uint(160),
                internal_type: None,
            },
            Param {
                name: "initializedTicksCrossed".to_string(),
                kind: ParamType::Uint(32),
                internal_type: None,
            },
            Param {
                name: "gasEstimate".to_string(),
                kind: ParamType::Uint(256),
                internal_type: None,
            },
        ],
        constant: None,
        state_mutability: StateMutability::NonPayable,
    };
    let bytes = function
        .encode_input(&[tuple])
        .map_err(|e| format!("failed to encode quoteExactInputSingle calldata: {e}"))?;
    Ok(format!("0x{}", hex::encode(bytes)))
}

/// Decode the first 32 bytes of a QuoterV2 response into the `amountOut` field.
/// The response is `(uint256, uint160, uint32, uint256)`; we only care about
/// the leading word.
pub fn decode_quoter_amount_out(response_hex: &str) -> Result<U256, String> {
    let bytes = hex_to_bytes(response_hex)?;
    if bytes.len() < 32 {
        return Err(format!(
            "quoter response too short: expected >=32 bytes, got {}",
            bytes.len()
        ));
    }
    Ok(U256::from_big_endian(&bytes[0..32]))
}

/// Hit the QuoterV2 contract via `eth_call` (read-only, no gas, no signing).
async fn quote_eth_to_toshi(amount_in_wei: U256) -> Result<U256, String> {
    let calldata = encode_quote_exact_input_single_eth_to_toshi(amount_in_wei)?;
    let response_hex: String = evm_rpc_call(
        EvmNetwork::BaseMainnet,
        "eth_call",
        json!([
            {
                "to": BASE_UNISWAP_V3_QUOTER,
                "data": calldata,
            },
            "latest"
        ]),
    )
    .await?;
    decode_quoter_amount_out(&response_hex)
}

fn exact_input_single_param_type() -> ParamType {
    ParamType::Tuple(vec![
        ParamType::Address,
        ParamType::Address,
        ParamType::Uint(24),
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(256),
        ParamType::Uint(160),
    ])
}

fn quote_exact_input_single_param_type() -> ParamType {
    ParamType::Tuple(vec![
        ParamType::Address,
        ParamType::Address,
        ParamType::Uint(256),
        ParamType::Uint(24),
        ParamType::Uint(160),
    ])
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Format a wei amount as decimal ETH for `amount_formatted`.
fn format_wei_as_eth(amount: U256) -> String {
    let wei_per_eth = U256::exp10(18);
    let whole = amount / wei_per_eth;
    let frac = amount % wei_per_eth;
    if frac.is_zero() {
        whole.to_string()
    } else {
        // Pad to 18 digits then strip trailing zeros.
        let frac_str = format!("{:018}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Prepare an ETH→TOSHI swap on Base. Calls QuoterV2 for the expected output,
/// applies a 1% slippage tolerance, builds the SwapRouter02 calldata, stores
/// the quote in the shared quote store, and returns it to the caller.
pub async fn prepare_buy_toshi_on_base(
    amount_in_wei: U256,
) -> Result<RpcOutcome<PreparedTransaction>, String> {
    if amount_in_wei.is_zero() {
        return Err("buy_toshi: amount_in_wei must be greater than zero".to_string());
    }
    let account = require_account(WalletChain::Evm).await?;
    let amount_out = quote_eth_to_toshi(amount_in_wei).await?;
    if amount_out.is_zero() {
        return Err(
            "buy_toshi: quoter returned amountOut=0 (no liquidity?); refusing to build calldata"
                .to_string(),
        );
    }
    let slippage_factor = U256::from(10_000u64 - DEFAULT_SLIPPAGE_BPS as u64);
    let amount_out_min = amount_out
        .checked_mul(slippage_factor)
        .ok_or_else(|| "buy_toshi: overflow applying slippage".to_string())?
        / U256::from(10_000u64);
    let calldata =
        encode_exact_input_single_eth_to_toshi(&account.address, amount_in_wei, amount_out_min)?;
    let now = now_ms();
    let quote = PreparedTransaction {
        quote_id: next_quote_id(),
        kind: PreparedKind::Swap,
        chain: WalletChain::Evm,
        evm_network: Some(EvmNetwork::BaseMainnet),
        from_address: account.address.clone(),
        to_address: BASE_UNISWAP_V3_ROUTER.to_string(),
        asset_symbol: "ETH".to_string(),
        amount_raw: amount_in_wei.to_string(),
        amount_formatted: format_wei_as_eth(amount_in_wei),
        receive_symbol: Some("TOSHI".to_string()),
        min_receive_raw: Some(amount_out_min.to_string()),
        calldata: Some(calldata),
        token_address: Some(BASE_TOSHI.to_string()),
        estimated_fee_raw: "0".to_string(),
        status: PreparedStatus::AwaitingConfirmation,
        created_at_ms: now,
        expires_at_ms: now + QUOTE_TTL_MS,
        notes: vec![format!(
            "Uniswap V3 (Base, 1% pool): {} ETH -> ~{} TOSHI (min {} after {}bps slippage)",
            format_wei_as_eth(amount_in_wei),
            amount_out,
            amount_out_min,
            DEFAULT_SLIPPAGE_BPS
        )],
    };
    debug!(
        "{LOG_PREFIX} prepare_buy_toshi quote_id={} amount_in_wei={} quoted_out={} min_out={}",
        quote.quote_id, amount_in_wei, amount_out, amount_out_min
    );
    Ok(RpcOutcome::new(
        store_quote(quote),
        vec!["wallet uniswap buy_toshi prepared".to_string()],
    ))
}

/// One-shot: prepare + execute. Routes through the normal `execute_prepared`
/// path so the `ApprovalGate` parks on confirmation when enabled.
pub async fn buy_toshi_on_base(amount_in_wei: U256) -> Result<RpcOutcome<ExecutionResult>, String> {
    let prepared = prepare_buy_toshi_on_base(amount_in_wei).await?.value;
    execute_prepared(ExecutePreparedParams {
        quote_id: prepared.quote_id.clone(),
        confirmed: true,
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_input_single_calldata_starts_with_known_selector() {
        // Selector for exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))
        // is 0x04e45aaf — well-known from Uniswap V3 SwapRouter02.
        let calldata = encode_exact_input_single_eth_to_toshi(
            "0x1111111111111111111111111111111111111111",
            U256::from(10_000_000_000_000_000u128), // 0.01 ETH
            U256::from(1_000u128),
        )
        .unwrap();
        assert!(
            calldata.starts_with("0x04e45aaf"),
            "expected exactInputSingle selector 0x04e45aaf, got {}",
            &calldata[..10]
        );
    }

    #[test]
    fn quote_exact_input_single_calldata_starts_with_known_selector() {
        // Selector for quoteExactInputSingle((address,address,uint256,uint24,uint160))
        // on QuoterV2 is 0xc6a5026a.
        let calldata =
            encode_quote_exact_input_single_eth_to_toshi(U256::from(10_000_000_000_000_000u128))
                .unwrap();
        assert!(
            calldata.starts_with("0xc6a5026a"),
            "expected quoteExactInputSingle selector 0xc6a5026a, got {}",
            &calldata[..10]
        );
    }

    #[test]
    fn decode_quoter_amount_out_reads_leading_word() {
        // Build a fake quoter response: amountOut = 42, then arbitrary padding
        // for the other three return slots.
        let mut bytes = vec![0u8; 32 * 4];
        bytes[31] = 42;
        let hex_str = format!("0x{}", hex::encode(&bytes));
        let parsed = decode_quoter_amount_out(&hex_str).unwrap();
        assert_eq!(parsed, U256::from(42u64));
    }

    #[test]
    fn decode_quoter_amount_out_rejects_short_response() {
        let err = decode_quoter_amount_out("0xdeadbeef").unwrap_err();
        assert!(err.contains("quoter response too short"), "got: {err}");
    }

    #[test]
    fn format_wei_as_eth_handles_round_amounts() {
        assert_eq!(format_wei_as_eth(U256::from(0u64)), "0");
        assert_eq!(format_wei_as_eth(U256::exp10(18)), "1");
        assert_eq!(format_wei_as_eth(U256::exp10(18) * 2), "2");
    }

    #[test]
    fn format_wei_as_eth_strips_trailing_zeros() {
        // 0.01 ETH = 10^16 wei
        assert_eq!(format_wei_as_eth(U256::exp10(16)), "0.01");
        // 0.001 ETH = 10^15 wei
        assert_eq!(format_wei_as_eth(U256::exp10(15)), "0.001");
    }
}
