use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{coins, from_json, Addr, Uint128};

use crate::contract::{execute, query};
use crate::msg::{ExecuteMsg, GetUserInfoResponse, QueryMsg};
use crate::tests::common::*;

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
fn test_deposit_with_unsupported_denom() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    // Try to deposit with unsupported denom
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &coins(deposit_amount, "unsupported"));
    let msg = ExecuteMsg::Deposit {};

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());

    // Verify it's the correct error type
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("UnsupportedDenom"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_deposit_with_multiple_denoms() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    // Try to deposit with multiple denoms
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());

    // Create multiple coins in the funds
    let funds = vec![
        coins(deposit_amount, DENOM)[0].clone(),
        coins(deposit_amount, "inj")[0].clone(),
    ];

    let info = message_info(&user_addr, &funds);
    let msg = ExecuteMsg::Deposit {};

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());

    // Verify it's the correct error type
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("MultipleDenoms"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_deposit_with_no_funds() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    // Try to deposit with no funds
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &[]);
    let msg = ExecuteMsg::Deposit {};

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());

    // Verify it's the correct error type
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("NoFunds"));
        }
        _ => panic!("Expected an error"),
    }
}
