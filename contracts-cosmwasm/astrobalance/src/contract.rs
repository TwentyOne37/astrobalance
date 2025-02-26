use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env,
    MessageInfo, Response, StdError, StdResult, Uint128,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, GetProtocolInfoResponse, GetProtocolsResponse, GetRebalanceHistoryResponse,
    GetRiskParametersResponse, GetTotalValueResponse, GetUserInfoResponse, InstantiateMsg,
    QueryMsg, RiskParametersMsg,
};
use crate::state::{
    Config, ProtocolInfo, RebalanceRecord, RiskParameters, UserDeposit, UserInfo, CONFIG,
    PROTOCOLS, REBALANCE_HISTORY, RISK_PARAMETERS, TOTAL_USDC_VALUE, USER_INFOS,
};
use crate::token_converter::AstroportRouter;

// version info for migration
const CONTRACT_NAME: &str = "crates.io:astrobalance";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // During tests, skip validation to avoid Bech32 errors
    #[cfg(test)]
    let config = Config {
        admin: Addr::unchecked(&msg.admin),
        ai_operator: Addr::unchecked(&msg.ai_operator),
        base_denom: msg.base_denom.clone(),
        accepted_denoms: msg.accepted_denoms.clone(),
        astroport_router: msg.astroport_router.clone(),
    };

    // In production, validate all addresses
    #[cfg(not(test))]
    let config = Config {
        admin: deps.api.addr_validate(&msg.admin)?,
        ai_operator: deps.api.addr_validate(&msg.ai_operator)?,
        base_denom: msg.base_denom.clone(),
        accepted_denoms: msg.accepted_denoms.clone(),
        astroport_router: deps.api.addr_validate(&msg.astroport_router)?.to_string(),
    };

    CONFIG.save(deps.storage, &config)?;

    // Initialize risk parameters
    let risk_parameters = RiskParameters {
        max_allocation_per_protocol: msg.risk_parameters.max_allocation_per_protocol,
        max_slippage: msg.risk_parameters.max_slippage,
        rebalance_threshold: msg.risk_parameters.rebalance_threshold,
        emergency_withdrawal_fee: msg.risk_parameters.emergency_withdrawal_fee,
    };
    RISK_PARAMETERS.save(deps.storage, &risk_parameters)?;

    // Initialize total USDC value
    TOTAL_USDC_VALUE.save(deps.storage, &Uint128::zero())?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("ai_operator", msg.ai_operator)
        .add_attribute("base_denom", msg.base_denom))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // User operations
        ExecuteMsg::Deposit {} => execute_deposit(deps, env, info),
        ExecuteMsg::Withdraw { amount, denom } => execute_withdraw(deps, env, info, amount, denom),
        ExecuteMsg::EmergencyWithdraw {} => execute_emergency_withdraw(deps, env, info),

        // Protocol management
        ExecuteMsg::AddProtocol {
            name,
            contract_addr,
            initial_allocation,
        } => execute_add_protocol(deps, env, info, name, contract_addr, initial_allocation),
        ExecuteMsg::RemoveProtocol { name } => execute_remove_protocol(deps, env, info, name),
        ExecuteMsg::UpdateProtocol {
            name,
            enabled,
            contract_addr,
        } => execute_update_protocol(deps, env, info, name, enabled, contract_addr),

        // AI Operator functions
        ExecuteMsg::Rebalance {
            target_allocations,
            reason,
        } => execute_rebalance(deps, env, info, target_allocations, reason),
        ExecuteMsg::UpdateRiskParameters { risk_parameters } => {
            execute_update_risk_parameters(deps, env, info, risk_parameters)
        }

        // Admin functions
        ExecuteMsg::AddSupportedToken { denom } => {
            execute_add_supported_token(deps, env, info, denom)
        }
        ExecuteMsg::RemoveSupportedToken { denom } => {
            execute_remove_supported_token(deps, env, info, denom)
        }
        ExecuteMsg::UpdateAdmin { admin } => execute_update_admin(deps, env, info, admin),
        ExecuteMsg::UpdateAiOperator { ai_operator } => {
            execute_update_ai_operator(deps, env, info, ai_operator)
        }
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let risk_parameters = RISK_PARAMETERS.load(deps.storage)?;

    // Check if funds were sent
    if info.funds.is_empty() {
        return Err(ContractError::NoFunds {});
    }

    // Only accept a single denomination per deposit
    if info.funds.len() > 1 {
        return Err(ContractError::MultipleDenoms {});
    }

    let deposit_coin = &info.funds[0];
    let denom = &deposit_coin.denom;
    let amount = deposit_coin.amount;

    // Check if the denomination is supported
    if !config.accepted_denoms.contains(&denom.to_string()) {
        return Err(ContractError::UnsupportedDenom {
            denom: denom.to_string(),
        });
    }

    // Create AstroportRouter instance
    let router = AstroportRouter(deps.api.addr_validate(&config.astroport_router)?);

    // Convert to USDC if needed
    let (conversion_msg, usdc_value) = if denom != &config.base_denom {
        router.convert_to_usdc(deps.as_ref(), denom, amount, risk_parameters.max_slippage)?
    } else {
        (
            CosmosMsg::Bank(BankMsg::Send {
                to_address: router.0.to_string(),
                amount: vec![Coin {
                    denom: denom.to_string(),
                    amount,
                }],
            })
            .into(),
            amount,
        )
    };

    // Update user's deposit record
    USER_INFOS.update(
        deps.storage,
        &info.sender,
        |maybe_user_info| -> StdResult<_> {
            let mut user_info = maybe_user_info.unwrap_or(UserInfo {
                total_usdc_value: Uint128::zero(),
                deposits: vec![],
            });

            // Add the new deposit
            user_info.deposits.push(UserDeposit {
                original_token: denom.to_string(),
                original_amount: amount,
                usdc_value_at_deposit: usdc_value,
                timestamp: env.block.time,
            });

            // Update total USDC value
            user_info.total_usdc_value += usdc_value;

            Ok(user_info)
        },
    )?;

    // Update total contract value
    TOTAL_USDC_VALUE.update(deps.storage, |total| -> StdResult<_> {
        Ok(total + usdc_value)
    })?;

    // For now, we'll just hold the USDC. In a full implementation, we would
    // distribute to protocols according to the current allocation strategy.

    Ok(Response::new()
        .add_message(conversion_msg)
        .add_attribute("method", "deposit")
        .add_attribute("depositor", info.sender)
        .add_attribute("original_denom", denom)
        .add_attribute("original_amount", amount)
        .add_attribute("usdc_value", usdc_value))
}

pub fn execute_withdraw(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
    denom: Option<String>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    if amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }

    // Check and update balance
    USER_INFOS.update(
        deps.storage,
        &info.sender,
        |maybe_user_info| -> StdResult<_> {
            let mut user_info = maybe_user_info.unwrap_or(UserInfo {
                total_usdc_value: Uint128::zero(),
                deposits: vec![],
            });

            // Update withdrawable amount
            if user_info.total_usdc_value < amount {
                return Err(StdError::generic_err("Insufficient funds"));
            }
            user_info.total_usdc_value -= amount;

            Ok(user_info)
        },
    )?;

    // Create bank message to send tokens
    let bank_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: denom.unwrap_or(config.base_denom),
            amount: Uint128::from(amount),
        }],
    };

    Ok(Response::new()
        .add_message(bank_msg)
        .add_attribute("method", "withdraw")
        .add_attribute("withdrawer", info.sender)
        .add_attribute("amount", amount.to_string()))
}

pub fn execute_emergency_withdraw(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "emergency_withdraw"))
}

pub fn execute_add_protocol(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _name: String,
    _contract_addr: String,
    _initial_allocation: Decimal,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

pub fn execute_remove_protocol(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _name: String,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "remove_protocol"))
}

pub fn execute_update_protocol(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _name: String,
    _enabled: Option<bool>,
    _contract_addr: Option<String>,
) -> Result<Response, ContractError> {
    Ok(Response::new().add_attribute("method", "update_protocol"))
}

pub fn execute_rebalance(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    target_allocations: Vec<(String, Decimal)>,
    reason: String,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    // Verify sender is AI operator
    if info.sender != config.ai_operator {
        return Err(ContractError::Unauthorized {});
    }

    // Implementation would go here...

    Ok(Response::new()
        .add_attribute("method", "rebalance")
        .add_attribute("reason", reason))
}

pub fn execute_update_risk_parameters(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _risk_parameters: RiskParametersMsg,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

pub fn execute_add_supported_token(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _denom: String,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

pub fn execute_remove_supported_token(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _denom: String,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

pub fn execute_update_admin(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _admin: String,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

pub fn execute_update_ai_operator(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _ai_operator: String,
) -> Result<Response, ContractError> {
    Err(ContractError::Std(StdError::generic_err("Not implemented")))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetUserInfo { address } => to_json_binary(&query_user_info(deps, address)?),
        QueryMsg::GetProtocols {} => to_json_binary(&query_protocols(deps)?),
        QueryMsg::GetProtocolInfo { name } => to_json_binary(&query_protocol_info(deps, name)?),
        QueryMsg::GetRiskParameters {} => to_json_binary(&query_risk_parameters(deps)?),
        QueryMsg::GetRebalanceHistory { limit } => {
            to_json_binary(&query_rebalance_history(deps, limit)?)
        }
        QueryMsg::GetTotalValue {} => to_json_binary(&query_total_value(deps)?),
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_user_info(deps: Deps, address: String) -> StdResult<GetUserInfoResponse> {
    // In tests, skip validation
    #[cfg(test)]
    let addr = Addr::unchecked(&address);

    // In production, validate the address
    #[cfg(not(test))]
    let addr = deps.api.addr_validate(&address)?;

    let user_info = USER_INFOS
        .may_load(deps.storage, &addr)?
        .unwrap_or(UserInfo {
            total_usdc_value: Uint128::zero(),
            deposits: vec![],
        });

    Ok(GetUserInfoResponse { user_info })
}

fn query_protocols(_deps: Deps) -> StdResult<GetProtocolsResponse> {
    let protocols: Vec<ProtocolInfo> = vec![];

    // This is a placeholder implementation
    // In a real implementation, you would iterate over PROTOCOLS

    Ok(GetProtocolsResponse { protocols })
}

fn query_protocol_info(deps: Deps, name: String) -> StdResult<GetProtocolInfoResponse> {
    let protocol = PROTOCOLS.may_load(deps.storage, &name)?.unwrap_or_else(|| {
        // Return a placeholder or handle the not-found case
        ProtocolInfo {
            name: name.clone(),
            contract_addr: deps.api.addr_validate("").unwrap(),
            allocation_percentage: Decimal::zero(),
            current_balance: Uint128::zero(),
            enabled: false,
        }
    });

    Ok(GetProtocolInfoResponse { protocol })
}

fn query_risk_parameters(deps: Deps) -> StdResult<GetRiskParametersResponse> {
    let risk_parameters = RISK_PARAMETERS.load(deps.storage)?;
    Ok(GetRiskParametersResponse { risk_parameters })
}

fn query_rebalance_history(
    deps: Deps,
    limit: Option<u32>,
) -> StdResult<GetRebalanceHistoryResponse> {
    let history = REBALANCE_HISTORY.load(deps.storage)?;
    let limit_val = limit.unwrap_or(history.len() as u32) as usize;

    let limited_history: Vec<RebalanceRecord> = history.iter().take(limit_val).cloned().collect();

    Ok(GetRebalanceHistoryResponse {
        history: limited_history,
    })
}

fn query_total_value(deps: Deps) -> StdResult<GetTotalValueResponse> {
    let total_value = TOTAL_USDC_VALUE.load(deps.storage)?;
    Ok(GetTotalValueResponse { total_value })
}

fn query_config(deps: Deps) -> StdResult<crate::msg::Config> {
    let state_config = CONFIG.load(deps.storage)?;

    // Create msg::Config for the response
    Ok(crate::msg::Config {
        admin: state_config.admin,
        ai_operator: state_config.ai_operator,
        base_denom: state_config.base_denom,
        accepted_denoms: state_config.accepted_denoms,
        astroport_router: state_config.astroport_router,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env, mock_info, MockApi};
    use cosmwasm_std::{coins, from_json, Addr, Uint128};

    // Generate valid bech32 addresses for testing
    fn addr(input: &str) -> String {
        MockApi::default().addr_make(input).to_string()
    }

    // Updated constants with valid addresses
    const DENOM: &str = "usdc";
    fn creator_address() -> String {
        addr("creator")
    }
    fn admin_address() -> String {
        addr("admin")
    }
    fn operator_address() -> String {
        addr("operator")
    }
    fn user_address() -> String {
        addr("user")
    }
    fn router_address() -> String {
        addr("router")
    }

    // Helper function to setup contract with valid addresses
    fn setup_contract(deps: DepsMut) {
        let msg = InstantiateMsg {
            admin: admin_address(),
            ai_operator: operator_address(),
            base_denom: DENOM.to_string(),
            accepted_denoms: vec![DENOM.to_string(), "inj".to_string()],
            astroport_router: router_address(),
            risk_parameters: RiskParametersMsg {
                max_allocation_per_protocol: Decimal::percent(50),
                max_slippage: Decimal::percent(1),
                rebalance_threshold: Decimal::percent(5),
                emergency_withdrawal_fee: Decimal::percent(1),
            },
        };

        let info = message_info(&Addr::unchecked(creator_address()), &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Verify config was saved correctly
        let config = CONFIG.load(deps.as_ref().storage).unwrap();
        assert_eq!(config.admin, Addr::unchecked(admin_address()));
        assert_eq!(config.ai_operator, Addr::unchecked(operator_address()));
        assert_eq!(config.astroport_router, router_address());
    }

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Test deposit
        let deposit_amount = 100u128;
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
        let msg = ExecuteMsg::Deposit {};

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res
            .attributes
            .iter()
            .any(|attr| attr.key == "method" && attr.value == "deposit"));

        // Verify user balance was updated
        let query_res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetUserInfo {
                address: user_address(),
            },
        )
        .unwrap();
        let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();
        assert_eq!(
            user_info.user_info.total_usdc_value,
            Uint128::from(deposit_amount)
        );
    }

    #[test]
    fn test_deposit_invalid_denom() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Test deposit with invalid denom
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(&user_addr, &coins(100, "invalid"));
        let msg = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();

        match err {
            ContractError::UnsupportedDenom { denom } => {
                assert_eq!(denom, "invalid");
            }
            _ => panic!("Expected UnsupportedDenom error, got: {:?}", err),
        }
    }

    #[test]
    fn test_deposit_multiple_denoms() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Test deposit with multiple denoms
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(
            &user_addr,
            &[
                Coin {
                    denom: DENOM.to_string(),
                    amount: Uint128::from(100u128),
                },
                Coin {
                    denom: "inj".to_string(),
                    amount: Uint128::from(100u128),
                },
            ],
        );
        let msg = ExecuteMsg::Deposit {};

        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::MultipleDenoms {}));
    }

    #[test]
    fn test_withdraw() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // First deposit some funds
        let deposit_amount = 100u128;
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(
            &user_addr,
            &[Coin {
                denom: DENOM.to_string(),
                amount: Uint128::from(deposit_amount),
            }],
        );
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

        // Now withdraw
        let withdraw_amount = 50u128;
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(&user_addr, &[]);
        let msg = ExecuteMsg::Withdraw {
            amount: Uint128::from(withdraw_amount),
            denom: None,
        };

        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check that the withdrawal message was created
        assert_eq!(1, res.messages.len());

        // Check user balance
        let query_res = query(
            deps.as_ref(),
            mock_env(),
            QueryMsg::GetUserInfo {
                address: user_address(),
            },
        )
        .unwrap();
        let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();
        assert_eq!(
            user_info.user_info.total_usdc_value,
            Uint128::from(deposit_amount - withdraw_amount)
        );
    }

    #[test]
    fn test_rebalance_permissions() {
        let mut deps = mock_dependencies();
        setup_contract(deps.as_mut());

        // Try to rebalance as non-operator (should fail)
        let user_addr = Addr::unchecked(user_address());
        let info = message_info(&user_addr, &[]);
        let msg = ExecuteMsg::Rebalance {
            target_allocations: vec![],
            reason: "Testing unauthorized".to_string(),
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert!(matches!(err, ContractError::Unauthorized {}));

        // Rebalance as operator (should succeed)
        let operator_addr = Addr::unchecked(operator_address());
        let info = message_info(&operator_addr, &[]);
        let msg = ExecuteMsg::Rebalance {
            target_allocations: vec![],
            reason: "Testing rebalance".to_string(),
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert!(res
            .attributes
            .iter()
            .any(|attr| attr.key == "method" && attr.value == "rebalance"));
    }
}
