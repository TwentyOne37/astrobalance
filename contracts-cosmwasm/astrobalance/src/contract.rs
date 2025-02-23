use cosmwasm_std::entry_point;
use cosmwasm_std::BankMsg;
use cosmwasm_std::Coin;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, OverflowError, OverflowOperation,
    Response, StdError, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{Config, ExecuteMsg, GetBalanceResponse, InstantiateMsg, QueryMsg};
use crate::state::{BALANCES, CONFIG};

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

    let admin = deps.api.addr_validate(&msg.admin)?;
    let ai_operator = deps.api.addr_validate(&msg.ai_operator)?;
    let accepted_denom = msg.accepted_denom.clone();

    let config = crate::state::Config {
        admin,
        ai_operator,
        accepted_denom: msg.accepted_denom,
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", msg.admin)
        .add_attribute("ai_operator", msg.ai_operator)
        .add_attribute("accepted_denom", accepted_denom))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {} => execute_deposit(deps, info),
        ExecuteMsg::Withdraw { amount } => execute_withdraw(deps, info, amount),
        ExecuteMsg::Rebalance {} => execute_rebalance(deps, info),
    }
}

pub fn execute_deposit(deps: DepsMut, info: MessageInfo) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;

    let amount = match info.funds.len() {
        0 => return Err(ContractError::NoFunds {}),
        1 => {
            let fund = &info.funds[0];
            if fund.denom != config.accepted_denom {
                return Err(ContractError::InvalidDenom {
                    expected: config.accepted_denom,
                    received: fund.denom.clone(),
                });
            }
            fund.amount.u128()
        }
        _ => return Err(ContractError::MultipleDenoms {}),
    };

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
    let config = CONFIG.load(deps.storage)?;

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

    // Create bank message to send tokens
    let bank_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: config.accepted_denom,
            amount: amount.into(),
        }],
    };

    Ok(Response::new()
        .add_message(bank_msg)
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
        accepted_denom: config.accepted_denom,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
    use cosmwasm_std::{coins, Addr, Coin, CosmosMsg, Uint128};

    const DENOM: &str = "usdc";

    fn setup_test_addresses() -> (String, String, String, String) {
        let deps = mock_dependencies();
        let creator = deps.api.addr_make("creator").to_string();
        let admin = deps.api.addr_make("admin").to_string();
        let operator = deps.api.addr_make("operator").to_string();
        let user = deps.api.addr_make("user").to_string();
        (creator, admin, operator, user)
    }

    fn setup_contract(deps: DepsMut, creator: &str, admin: &str, operator: &str) {
        let msg = InstantiateMsg {
            admin: admin.to_string(),
            ai_operator: operator.to_string(),
            accepted_denom: DENOM.to_string(),
        };
        let creator_addr = Addr::unchecked(creator);
        let info = message_info(&creator_addr, &[]);
        instantiate(deps, mock_env(), info, msg).unwrap();
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, _) = setup_test_addresses();
        let creator_addr = Addr::unchecked(&creator);

        let msg = InstantiateMsg {
            admin: admin.clone(),
            ai_operator: operator.clone(),
            accepted_denom: DENOM.to_string(),
        };
        let info = message_info(&creator_addr, &[]);

        // Execute instantiate
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // Check config
        let config = query_config(deps.as_ref()).unwrap();
        assert_eq!(config.admin, Addr::unchecked(admin));
        assert_eq!(config.ai_operator, Addr::unchecked(operator));
        assert_eq!(config.accepted_denom, DENOM);
    }

    #[test]
    fn test_deposit() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        let user_addr = Addr::unchecked(&user);
        let deposit_amount = 100u128;
        let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
        let msg = ExecuteMsg::Deposit {};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 3);

        // Query balance
        let balance = query_balance(deps.as_ref(), user.clone()).unwrap();
        assert_eq!(balance.balance, deposit_amount);
    }

    #[test]
    fn test_deposit_invalid_denom() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        // Try to deposit with wrong denom
        let user_addr = Addr::unchecked(&user);
        let info = message_info(&user_addr, &coins(100, "invalid"));
        let msg = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(
            err,
            ContractError::InvalidDenom {
                expected: DENOM.to_string(),
                received: "invalid".to_string()
            }
        );
    }

    #[test]
    fn test_deposit_no_funds() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        // Try to deposit with no funds
        let user_addr = Addr::unchecked(&user);
        let info = message_info(&user_addr, &[]);
        let msg = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::NoFunds {});
    }

    #[test]
    fn test_deposit_multiple_denoms() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        // Try to deposit multiple denoms
        let user_addr = Addr::unchecked(&user);
        let info = message_info(
            &user_addr,
            &[Coin::new(100u128, DENOM), Coin::new(100u128, "other")],
        );
        let msg = ExecuteMsg::Deposit {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::MultipleDenoms {});
    }

    #[test]
    fn test_withdraw() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        // First deposit
        let deposit_amount = 100u128;
        let user_addr = Addr::unchecked(&user);
        let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
        let msg = ExecuteMsg::Deposit {};
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Test withdraw
        let withdraw_amount = 50u128;
        let info = message_info(&user_addr, &[]);
        let msg = ExecuteMsg::Withdraw {
            amount: withdraw_amount,
        };
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // Check the bank message was created correctly
        assert_eq!(res.messages.len(), 1);
        match &res.messages[0].msg {
            CosmosMsg::Bank(BankMsg::Send { to_address, amount }) => {
                assert_eq!(&Addr::unchecked(to_address), &user_addr);
                assert_eq!(amount.len(), 1);
                assert_eq!(amount[0].denom, DENOM);
                assert_eq!(amount[0].amount, Uint128::from(withdraw_amount));
            }
            _ => panic!("Expected BankMsg::Send"),
        }

        // Query updated balance
        let balance = query_balance(deps.as_ref(), user.clone()).unwrap();
        assert_eq!(balance.balance, deposit_amount - withdraw_amount);
    }

    #[test]
    fn test_rebalance_permissions() {
        let mut deps = mock_dependencies();
        let (creator, admin, operator, user) = setup_test_addresses();
        setup_contract(deps.as_mut(), &creator, &admin, &operator);

        // Try to rebalance as non-operator
        let user_addr = Addr::unchecked(&user);
        let info = message_info(&user_addr, &[]);
        let msg = ExecuteMsg::Rebalance {};
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // Rebalance as operator
        let operator_addr = Addr::unchecked(&operator);
        let info = message_info(&operator_addr, &[]);
        let msg = ExecuteMsg::Rebalance {};
        let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(res.attributes.len(), 1);
    }
}
