use cosmwasm_std::testing::{message_info, mock_env, MockApi, MockQuerier, MockStorage};
use cosmwasm_std::{
    to_json_binary, Addr, ContractResult, Decimal, DepsMut, Empty, OwnedDeps, SystemError,
    SystemResult, Uint128,
};

use crate::contract::{execute, instantiate};
use crate::msg::{ExecuteMsg, InstantiateMsg, RiskParametersMsg};
use crate::token_converter::SimulateSwapResponse;

// Use our test models since we're using Option 2
use self::test_models::*;

// Import test models for protocol response mocking
pub mod test_models {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Decimal, Uint128};

    #[cw_serde]
    pub struct HelixBalanceResponse {
        pub amount: Uint128,
    }

    #[cw_serde]
    pub struct HelixApyResponse {
        pub apy: Decimal,
    }

    #[cw_serde]
    pub struct HydroBalanceResponse {
        pub supplied_amount: Uint128,
    }

    #[cw_serde]
    pub struct HydroLendingRateResponse {
        pub rate: Decimal,
    }

    #[cw_serde]
    pub struct NeptuneStakedBalanceResponse {
        pub amount: Uint128,
    }

    #[cw_serde]
    pub struct NeptuneStakingRateResponse {
        pub apy: Decimal,
    }
}

// Constants for testing
pub const DENOM: &str = "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5";

// Generate valid bech32 addresses for testing
pub fn addr(input: &str) -> String {
    MockApi::default().addr_make(input).to_string()
}

pub fn creator_address() -> String {
    addr("creator")
}

pub fn admin_address() -> String {
    addr("admin")
}

pub fn operator_address() -> String {
    addr("operator")
}

pub fn user_address() -> String {
    addr("user")
}

pub fn router_address() -> String {
    addr("router")
}

// Helper function to setup contract with valid addresses
pub fn setup_contract(deps: DepsMut) {
    let msg = InstantiateMsg {
        admin: admin_address(),
        ai_operator: operator_address(),
        base_denom: "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(), // USDT
        accepted_denoms: vec![
            "peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5".to_string(),
            "inj".to_string(),
        ],
        astroport_router: router_address(),
        risk_parameters: RiskParametersMsg {
            max_allocation_per_protocol: Decimal::percent(100), // Allow up to 100% allocation
            max_slippage: Decimal::percent(1),
            rebalance_threshold: Decimal::percent(5),
            emergency_withdrawal_fee: Decimal::percent(1),
        },
    };

    // Fix here: Addr::unchecked instead of using the string directly
    let info = message_info(&Addr::unchecked(creator_address()), &[]);
    instantiate(deps, mock_env(), info, msg).unwrap();
}

// Mock protocol adapter response for testing
pub fn mock_protocol_response(deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>) {
    deps.querier.update_wasm(|query| {
        match query {
            // Mock balance query for Helix
            cosmwasm_std::WasmQuery::Smart { contract_addr, msg } => {
                // Fix: Use Binary's as_slice() for UTF-8 conversion
                let msg_str = String::from_utf8(msg.as_slice().to_vec()).unwrap_or_default();

                if contract_addr.contains("helix") && msg_str.contains("Balance") {
                    let mock_response = HelixBalanceResponse {
                        amount: Uint128::from(100u128),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                }
                // Mock APY query for Helix
                else if contract_addr.contains("helix") && msg_str.contains("Apy") {
                    let mock_response = HelixApyResponse {
                        apy: Decimal::percent(5),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                }
                // Mock Hydro queries
                else if contract_addr.contains("hydro") && msg_str.contains("LenderBalance") {
                    let mock_response = HydroBalanceResponse {
                        supplied_amount: Uint128::from(150u128),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                } else if contract_addr.contains("hydro") && msg_str.contains("LendingRate") {
                    let mock_response = HydroLendingRateResponse {
                        rate: Decimal::percent(7),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                }
                // Mock Neptune queries
                else if contract_addr.contains("neptune") && msg_str.contains("StakedBalance") {
                    let mock_response = NeptuneStakedBalanceResponse {
                        amount: Uint128::from(200u128),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                } else if contract_addr.contains("neptune") && msg_str.contains("StakingRate") {
                    let mock_response = NeptuneStakingRateResponse {
                        apy: Decimal::percent(9),
                    };
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&mock_response).unwrap()))
                }
                // Mock Astroport router responses for SimulateSwapOperations
                else if contract_addr.contains("router")
                    && msg_str.contains("SimulateSwapOperations")
                {
                    // Check if it's a conversion to or from USDC
                    if msg_str.contains("inj") && msg_str.contains("usdc") {
                        // This simulates converting INJ to USDC or USDC to INJ
                        // For tests, we'll use a simple 1:10 ratio (1 INJ = 10 USDC)
                        let sim_response = if msg_str.contains(r#""denom":"inj""#)
                            && msg_str.contains(r#""denom":"usdc""#)
                        {
                            // INJ to USDC (multiply by 10)
                            let amount_str = msg_str
                                .split("offer_amount\":")
                                .nth(1)
                                .unwrap_or("0")
                                .split(",")
                                .next()
                                .unwrap_or("0");
                            let amount = amount_str.trim().parse::<u128>().unwrap_or(0);
                            SimulateSwapResponse {
                                amount: Uint128::from(amount * 10),
                            }
                        } else {
                            // USDC to INJ (divide by 10)
                            let amount_str = msg_str
                                .split("offer_amount\":")
                                .nth(1)
                                .unwrap_or("0")
                                .split(",")
                                .next()
                                .unwrap_or("0");
                            let amount = amount_str.trim().parse::<u128>().unwrap_or(0);
                            SimulateSwapResponse {
                                amount: Uint128::from(amount / 10),
                            }
                        };
                        SystemResult::Ok(ContractResult::Ok(to_json_binary(&sim_response).unwrap()))
                    } else {
                        // For any other token conversion
                        let mock_response = SimulateSwapResponse {
                            amount: Uint128::from(98u128), // 2% price impact
                        };
                        SystemResult::Ok(ContractResult::Ok(
                            to_json_binary(&mock_response).unwrap(),
                        ))
                    }
                }
                // Mock ExecuteSwapOperations (needed for actual swap execution)
                else if contract_addr.contains("router")
                    && msg_str.contains("ExecuteSwapOperations")
                {
                    // Just return an empty response as this is an execute message
                    SystemResult::Ok(ContractResult::Ok(to_json_binary(&"").unwrap()))
                } else {
                    // Fix: Wrap error in correct type
                    SystemResult::Err(SystemError::InvalidRequest {
                        error: "Unexpected wasm query type".to_string(),
                        request: Default::default(),
                    })
                }
            }
            _ => SystemResult::Err(SystemError::InvalidRequest {
                error: "Unexpected wasm query type".to_string(),
                request: Default::default(),
            }),
        }
    });
}

// Helper to set up contract with protocols
pub fn setup_contract_with_protocols(
    deps: &mut OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
) -> Addr {
    setup_contract(deps.as_mut());
    let admin = Addr::unchecked(admin_address());

    // Add Helix protocol
    let add_helix_msg = ExecuteMsg::AddProtocol {
        name: "helix".to_string(),
        contract_addr: format!("{}helix", addr("contract_")),
        initial_allocation: Decimal::percent(30),
    };
    execute(
        deps.as_mut(),
        mock_env(),
        // Fix: Pass Addr reference instead of &str
        message_info(&admin, &[]),
        add_helix_msg,
    )
    .unwrap();

    // Add Hydro protocol
    let add_hydro_msg = ExecuteMsg::AddProtocol {
        name: "hydro".to_string(),
        contract_addr: format!("{}hydro", addr("contract_")),
        initial_allocation: Decimal::percent(30),
    };
    execute(
        deps.as_mut(),
        mock_env(),
        // Fix: Pass Addr reference instead of &str
        message_info(&admin, &[]),
        add_hydro_msg,
    )
    .unwrap();

    // Add Neptune protocol
    let add_neptune_msg = ExecuteMsg::AddProtocol {
        name: "neptune".to_string(),
        contract_addr: format!("{}neptune", addr("contract_")),
        initial_allocation: Decimal::percent(40),
    };
    execute(
        deps.as_mut(),
        mock_env(),
        // Fix: Pass Addr reference instead of &str
        message_info(&admin, &[]),
        add_neptune_msg,
    )
    .unwrap();

    admin
}
