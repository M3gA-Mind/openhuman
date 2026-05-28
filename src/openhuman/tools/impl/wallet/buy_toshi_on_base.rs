use crate::openhuman::tools::traits::{Tool, ToolCallOptions, ToolResult};
use crate::openhuman::wallet;
use async_trait::async_trait;
use ethers_core::types::U256;
use serde::Deserialize;
use serde_json::json;

pub struct WalletBuyToshiOnBaseTool;

impl WalletBuyToshiOnBaseTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BuyToshiArgs {
    eth_amount_wei: String,
}

#[async_trait]
impl Tool for WalletBuyToshiOnBaseTool {
    fn name(&self) -> &str {
        "wallet_buy_toshi_on_base"
    }

    fn description(&self) -> &str {
        "Buy TOSHI on Base by swapping ETH via Uniswap V3 (1% WETH/TOSHI pool). One-shot: \
         prepares the quote (QuoterV2 + 1% slippage), signs from the user's locally-derived \
         EVM key, and broadcasts to https://mainnet.base.org. Returns the transaction hash and \
         a Basescan explorer URL. Real ETH leaves the wallet — every call should be the result \
         of an explicit user instruction. Common amounts: 0.001 ETH = '1000000000000000', \
         0.01 ETH = '10000000000000000'."
    }

    fn parameters_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "ethAmountWei": {
                    "type": "string",
                    "description": "Amount of ETH to spend, in wei (decimal string). 1 ETH = 10^18 wei. Example: '10000000000000000' = 0.01 ETH."
                }
            },
            "required": ["ethAmountWei"],
            "additionalProperties": false
        })
    }

    fn external_effect(&self) -> bool {
        // Real ETH leaves the wallet and lands on-chain — route every
        // invocation through ApprovalGate so the user explicitly confirms
        // before signing.
        true
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
        let parsed: BuyToshiArgs = match serde_json::from_value(args) {
            Ok(value) => value,
            Err(e) => {
                log::debug!("[wallet_buy_toshi_on_base] invalid arguments: {e}");
                return Ok(ToolResult::error(format!("invalid arguments: {e}")));
            }
        };
        let amount = match U256::from_dec_str(parsed.eth_amount_wei.trim()) {
            Ok(value) => value,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "invalid ethAmountWei '{}': {e}",
                    parsed.eth_amount_wei
                )));
            }
        };
        log::debug!(
            "[wallet_buy_toshi_on_base] executing eth_wei={}",
            parsed.eth_amount_wei
        );

        match wallet::buy_toshi_on_base(amount).await {
            Ok(outcome) => {
                let json_str = serde_json::to_string_pretty(&outcome.value)?;
                log::debug!("[wallet_buy_toshi_on_base] success");
                Ok(ToolResult::success(json_str))
            }
            Err(e) => {
                log::warn!("[wallet_buy_toshi_on_base] failed: {e}");
                Ok(ToolResult::error(e))
            }
        }
    }
}
