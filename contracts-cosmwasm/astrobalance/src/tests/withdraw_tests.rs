use cosmwasm_std::testing::{message_info, mock_dependencies, mock_env};
use cosmwasm_std::{coins, from_json, Addr, Uint128};

use crate::contract::{execute, query};
use crate::msg::{ExecuteMsg, GetUserInfoResponse, QueryMsg};
use crate::tests::common::*;

#[test]
fn test_withdraw() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    // First deposit some funds
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // Now withdraw half
    let withdraw_amount = 50u128;
    let info = message_info(&user_addr, &[]);
    let msg = ExecuteMsg::Withdraw {
        amount: Uint128::from(withdraw_amount),
        denom: None, // Use default denom
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "withdraw"));

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
        Uint128::from(deposit_amount - withdraw_amount)
    );
}

#[test]
fn test_withdraw_insufficient_funds() {
    let mut deps = mock_dependencies();
    setup_contract(deps.as_mut());

    // First deposit some funds
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // Try to withdraw more than deposited
    let withdraw_amount = 150u128;
    let info = message_info(&user_addr, &[]);
    let msg = ExecuteMsg::Withdraw {
        amount: Uint128::from(withdraw_amount),
        denom: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());

    match res {
        Err(e) => {
            // Verify it's the correct error type
            assert!(format!("{:?}", e).contains("InsufficientFunds"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
#[ignore = "Token conversion via Astroport router is difficult to mock correctly"]
fn test_withdraw_with_specific_denom() {
    // This test is ignored because mocking the Astroport router for token conversion
    // (from USDC to INJ) is complex and requires detailed parsing of Binary messages.
    // In a real environment, the router would handle this conversion properly.
    // For local testing, we would need a more sophisticated mock that can parse
    // the exact binary format of the router requests/responses.

    let mut deps = mock_dependencies();
    mock_protocol_response(&mut deps);
    setup_contract(deps.as_mut());

    // First deposit some funds
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // Now withdraw as INJ
    let withdraw_amount = 50u128;
    let info = message_info(&user_addr, &[]);
    let msg = ExecuteMsg::Withdraw {
        amount: Uint128::from(withdraw_amount),
        denom: Some("inj".to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Check that the specific denom is used in response
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "denom" && attr.value == "inj"));
}

#[test]
fn test_emergency_withdraw() {
    let mut deps = mock_dependencies();
    mock_protocol_response(&mut deps);
    setup_contract(deps.as_mut());

    // First deposit some funds
    let deposit_amount = 100u128;
    let user_addr = Addr::unchecked(user_address());
    let info = message_info(&user_addr, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // Execute emergency withdraw
    let info = message_info(&user_addr, &[]);
    let msg = ExecuteMsg::EmergencyWithdraw {};

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Check that fee was applied (1%)
    let fee_attribute = res
        .attributes
        .iter()
        .find(|attr| attr.key == "fee_amount")
        .expect("Fee amount attribute missing");

    assert_eq!(fee_attribute.value, "1"); // 1% of 100 is 1

    // Verify user balance is now zero
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user_address(),
        },
    )
    .unwrap();
    let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();
    assert_eq!(user_info.user_info.total_usdc_value, Uint128::zero());
}
