#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, OverflowError,
    OverflowOperation, Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{Config, ExecuteMsg, GetBalanceResponse, InstantiateMsg, QueryMsg};
use crate::state::{Config as ConfigState, BALANCES, CONFIG};

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
    // Set contract version for future migrations
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // Convert String addresses to Addr type
    let admin = deps.api.addr_validate(&msg.admin)?;
    let ai_operator = deps.api.addr_validate(&msg.ai_operator)?;

    // Save config to state
    let config = ConfigState { admin, ai_operator };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("ai_operator", msg.ai_operator))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { amount } => execute_deposit(deps, info, amount),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, info, amount),
        ExecuteMsg::Rebalance { .. } => execute_rebalance(deps, info),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    info: MessageInfo,
    amount: u128,
) -> Result<Response, ContractError> {
    if amount == 0 {
        return Err(ContractError::InvalidAmount {});
    }

    // Update balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<u128>| -> StdResult<_> {
            Ok(balance
                .unwrap_or_default()
                .checked_add(amount)
                .ok_or(StdError::overflow(OverflowError {
                    operation: OverflowOperation::Add,
                }))?)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "deposit")
        .add_attribute("depositor", info.sender)
        .add_attribute("amount", amount.to_string()))
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    amount: u128,
) -> Result<Response, ContractError> {
    if amount == 0 {
        return Err(ContractError::InvalidAmount {});
    }

    // Check and update balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<u128>| -> Result<_, ContractError> {
            let current_balance = balance.unwrap_or_default();
            if current_balance < amount {
                return Err(ContractError::InsufficientFunds {});
            }
            Ok(current_balance - amount)
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("withdrawer", info.sender)
        .add_attribute("amount", amount.to_string()))
}

pub fn execute_rebalance(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    // Only AI operator can rebalance
    let config = CONFIG.load(deps.storage)?;
    if info.sender != config.ai_operator {
        return Err(ContractError::Unauthorized {});
    }

    // Add rebalancing logic here
    Ok(Response::new().add_attribute("method", "rebalance"))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetBalance { address } => to_json_binary(&query_balance(deps, address)?),
        QueryMsg::GetConfig {} => to_json_binary(&query_config(deps)?),
    }
}

fn query_balance(deps: Deps, address: String) -> StdResult<GetBalanceResponse> {
    let addr = deps.api.addr_validate(&address)?;
    let balance = BALANCES.may_load(deps.storage, &addr)?.unwrap_or_default();
    Ok(GetBalanceResponse { balance })
}

fn query_config(deps: Deps) -> StdResult<Config> {
    let config = CONFIG.load(deps.storage)?;
    Ok(Config {
        admin: config.admin,
        ai_operator: config.ai_operator,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    fn setup_test_addresses() -> (String, String, String, String) {
        let deps = mock_dependencies();
        let creator = deps.api.addr_make("creator").to_string();
        let admin = deps.api.addr_make("admin").to_string();
        let operator = deps.api.addr_make("operator").to_string();
        let user = deps.api.addr_make("user").to_string();
        (creator, admin, operator, user)
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, _) = setup_test_addresses();
        let info = mock_info(&creator, &[]);

        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
        };

        // Execute instantiate
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check config
        let config = query_config(deps.as_ref()).unwrap();
        assert_eq!(config.admin, Addr::unchecked(admin));
        assert_eq!(config.ai_operator, Addr::unchecked(operator));
    }

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();

        // Instantiate contract
        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
        };
        let info = mock_info(&creator, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test deposit
        let deposit_amount = 100u128;
        let info = mock_info(&user, &[]);
        let msg = ExecuteMsg::Deposit {
            amount: deposit_amount,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3);

        // Query balance
        let balance = query_balance(deps.as_ref(), user.clone()).unwrap();
        assert_eq!(balance.balance, deposit_amount);
    }

    #[test]
    fn test_withdraw() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();

        // Instantiate contract
        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
        };
        let info = mock_info(&creator, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // First deposit
        let deposit_amount = 100u128;
        let info = mock_info(&user, &[]);
        let msg = ExecuteMsg::Deposit {
            amount: deposit_amount,
        };
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test withdraw
        let withdraw_amount = 50u128;
        let info = mock_info(&user, &[]);
        let msg = ExecuteMsg::Withdraw {
            amount: withdraw_amount,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3);

        // Query updated balance
        let balance = query_balance(deps.as_ref(), user.clone()).unwrap();
        assert_eq!(balance.balance, deposit_amount - withdraw_amount);
    }

    #[test]
    fn test_withdraw_insufficient_funds() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();

        // Instantiate contract
        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
        };
        let info = mock_info(&creator, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try to withdraw without deposit
        let withdraw_amount = 50u128;
        let info = mock_info(&user, &[]);
        let msg = ExecuteMsg::Withdraw {
            amount: withdraw_amount,
        };
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::InsufficientFunds {});
    }

    #[test]
    fn test_rebalance_permissions() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();

        // Instantiate contract
        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
        };
        let info = mock_info(&creator, &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Try to rebalance as non-operator
        let info = mock_info(&user, &[]);
        let msg = ExecuteMsg::Rebalance {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // Rebalance as operator
        let info = mock_info(&operator, &[]);
        let msg = ExecuteMsg::Rebalance {};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 1);
    }
}
