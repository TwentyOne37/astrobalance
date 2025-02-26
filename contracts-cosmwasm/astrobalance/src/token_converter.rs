use crate::error::ContractError;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Coin, CosmosMsg, Decimal, Deps, StdResult, Uint128, WasmMsg,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct AstroportRouter(pub Addr);

impl AstroportRouter {
    // Convert any supported token to USDC
    pub fn convert_to_usdc(
        &self,
        deps: Deps,
        denom: &str,
        amount: Uint128,
        max_slippage: Decimal,
    ) -> StdResult<(CosmosMsg, Uint128)> {
        // If already USDC, no conversion needed
        if denom == "usdc" {
            return Ok((
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: self.0.to_string(),
                    amount: vec![Coin {
                        denom: denom.to_string(),
                        amount,
                    }],
                }),
                amount,
            ));
        }

        // Query Astroport for estimated return
        let simulate_swap: SimulateSwapResponse = deps.querier.query_wasm_smart(
            self.0.to_string(),
            &astroport::QueryMsg::SimulateSwapOperations {
                offer_amount: amount,
                operations: vec![astroport::SwapOperation::AstroSwap {
                    offer_asset_info: astroport::AssetInfo::NativeToken {
                        denom: denom.to_string(),
                    },
                    ask_asset_info: astroport::AssetInfo::NativeToken {
                        denom: "usdc".to_string(),
                    },
                }],
            },
        )?;

        // Calculate minimum expected with slippage
        let min_expected = simulate_swap.amount.multiply_ratio(
            Uint128::new(1_000_000) - max_slippage.atomics(),
            Uint128::new(1_000_000),
        );

        // Create the swap message
        let swap_msg = WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&astroport::ExecuteMsg::ExecuteSwapOperations {
                operations: vec![astroport::SwapOperation::AstroSwap {
                    offer_asset_info: astroport::AssetInfo::NativeToken {
                        denom: denom.to_string(),
                    },
                    ask_asset_info: astroport::AssetInfo::NativeToken {
                        denom: "usdc".to_string(),
                    },
                }],
                minimum_receive: Some(min_expected),
            })?,
            funds: vec![Coin {
                denom: denom.to_string(),
                amount,
            }],
        };

        Ok((CosmosMsg::Wasm(swap_msg), simulate_swap.amount))
    }

    // Convert USDC to requested token
    pub fn convert_from_usdc(
        &self,
        deps: Deps,
        to_denom: &str,
        amount: Uint128,
        max_slippage: Decimal,
    ) -> StdResult<(CosmosMsg, Uint128)> {
        // If requesting USDC, no conversion needed
        if to_denom == "usdc" {
            return Ok((
                CosmosMsg::Bank(BankMsg::Send {
                    to_address: self.0.to_string(),
                    amount: vec![Coin {
                        denom: to_denom.to_string(),
                        amount,
                    }],
                }),
                amount,
            ));
        }

        // Query Astroport for estimated return
        let simulate_swap: SimulateSwapResponse = deps.querier.query_wasm_smart(
            self.0.to_string(),
            &astroport::QueryMsg::SimulateSwapOperations {
                offer_amount: amount,
                operations: vec![astroport::SwapOperation::AstroSwap {
                    offer_asset_info: astroport::AssetInfo::NativeToken {
                        denom: "usdc".to_string(),
                    },
                    ask_asset_info: astroport::AssetInfo::NativeToken {
                        denom: to_denom.to_string(),
                    },
                }],
            },
        )?;

        // Calculate minimum expected with slippage
        let min_expected = simulate_swap.amount.multiply_ratio(
            Uint128::new(1_000_000) - max_slippage.atomics(),
            Uint128::new(1_000_000),
        );

        // Create the swap message
        let swap_msg = WasmMsg::Execute {
            contract_addr: self.0.to_string(),
            msg: to_json_binary(&astroport::ExecuteMsg::ExecuteSwapOperations {
                operations: vec![astroport::SwapOperation::AstroSwap {
                    offer_asset_info: astroport::AssetInfo::NativeToken {
                        denom: "usdc".to_string(),
                    },
                    ask_asset_info: astroport::AssetInfo::NativeToken {
                        denom: to_denom.to_string(),
                    },
                }],
                minimum_receive: Some(min_expected),
            })?,
            funds: vec![Coin {
                denom: "usdc".to_string(),
                amount,
            }],
        };

        Ok((CosmosMsg::Wasm(swap_msg), simulate_swap.amount))
    }

    // Get price quote for UI preview - doesn't execute a swap
    pub fn get_price_quote(
        &self,
        deps: Deps,
        from_denom: &str,
        to_denom: &str,
        amount: Uint128,
    ) -> Result<Uint128, ContractError> {
        // If same token, 1:1 rate
        if from_denom == to_denom {
            return Ok(amount);
        }

        // Determine swap direction
        let (offer_denom, ask_denom, _is_to_usdc) = if to_denom == "usdc" {
            (from_denom, "usdc", true)
        } else if from_denom == "usdc" {
            ("usdc", to_denom, false)
        } else {
            // For non-USDC pairs, we need to do a double hop through USDC
            // First get quote from from_denom -> USDC
            let usdc_amount = self.get_price_quote(deps, from_denom, "usdc", amount)?;
            // Then get quote from USDC -> to_denom
            return self.get_price_quote(deps, "usdc", to_denom, usdc_amount);
        };

        // Query Astroport for simulated swap
        let simulate_result: StdResult<SimulateSwapResponse> = deps.querier.query_wasm_smart(
            self.0.to_string(),
            &astroport::QueryMsg::SimulateSwapOperations {
                offer_amount: amount,
                operations: vec![astroport::SwapOperation::AstroSwap {
                    offer_asset_info: astroport::AssetInfo::NativeToken {
                        denom: offer_denom.to_string(),
                    },
                    ask_asset_info: astroport::AssetInfo::NativeToken {
                        denom: ask_denom.to_string(),
                    },
                }],
            },
        );

        match simulate_result {
            Ok(response) => Ok(response.amount),
            Err(err) => Err(ContractError::ConversionError {
                error: format!("Failed to get price quote: {}", err),
            }),
        }
    }

    // Safe version with error handling for contract usage
    pub fn safe_convert_to_usdc(
        &self,
        deps: Deps,
        denom: &str,
        amount: Uint128,
        max_slippage: Decimal,
    ) -> Result<(CosmosMsg, Uint128), ContractError> {
        if amount.is_zero() {
            return Err(ContractError::InvalidAmount {});
        }

        match self.convert_to_usdc(deps, denom, amount, max_slippage) {
            Ok(result) => Ok(result),
            Err(err) => Err(ContractError::ConversionError {
                error: format!("Failed to convert to USDC: {}", err),
            }),
        }
    }

    // Safe version with error handling for contract usage
    pub fn safe_convert_from_usdc(
        &self,
        deps: Deps,
        to_denom: &str,
        amount: Uint128,
        max_slippage: Decimal,
    ) -> Result<(CosmosMsg, Uint128), ContractError> {
        if amount.is_zero() {
            return Err(ContractError::InvalidAmount {});
        }

        match self.convert_from_usdc(deps, to_denom, amount, max_slippage) {
            Ok(result) => Ok(result),
            Err(err) => Err(ContractError::ConversionError {
                error: format!("Failed to convert from USDC: {}", err),
            }),
        }
    }
}

// Astroport interface definitions - would be replaced with actual imports
pub mod astroport {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::Uint128;

    #[cw_serde]
    pub enum AssetInfo {
        NativeToken { denom: String },
        Token { contract_addr: String },
    }

    #[cw_serde]
    pub enum SwapOperation {
        AstroSwap {
            offer_asset_info: AssetInfo,
            ask_asset_info: AssetInfo,
        },
        // Other swap types...
    }

    #[cw_serde]
    pub enum ExecuteMsg {
        ExecuteSwapOperations {
            operations: Vec<SwapOperation>,
            minimum_receive: Option<Uint128>,
        },
    }

    #[cw_serde]
    pub enum QueryMsg {
        SimulateSwapOperations {
            offer_amount: Uint128,
            operations: Vec<SwapOperation>,
        },
    }
}

#[cw_serde]
pub struct SimulateSwapResponse {
    pub amount: Uint128,
}
