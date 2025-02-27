use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, Addr, Decimal, Empty, OwnedDeps, Uint128};

use crate::contract::{execute, query};
use crate::msg::{
    ExecuteMsg, GetProtocolInfoResponse, GetProtocolsResponse, GetRebalanceHistoryResponse,
    GetTotalValueResponse, GetUserInfoResponse, QueryMsg,
};
use crate::tests::common::*;
use crate::tests::protocol_tests::setup_test_protocols;

// Helper function to set up a test environment with protocols
fn setup_integration_test() -> (
    OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    Addr,
    Addr,
    Addr,
) {
    let mut deps = mock_dependencies();
    mock_protocol_response(&mut deps);
    setup_contract(deps.as_mut());
    setup_test_protocols(&mut deps);

    let admin = Addr::unchecked(admin_address());
    let operator = Addr::unchecked(operator_address());
    let user = Addr::unchecked(user_address());

    (deps, admin, operator, user)
}

#[test]
fn test_full_lifecycle() {
    let (mut deps, admin, operator, user) = setup_integration_test();

    // Step 1: User deposits funds
    let deposit_amount = 1000u128;
    let info = message_info(&user, &coins(deposit_amount, DENOM));
    let res = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "deposit"));

    // Verify user info after deposit
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user.to_string(),
        },
    )
    .unwrap();
    let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();

    assert_eq!(
        user_info.user_info.total_usdc_value,
        Uint128::new(deposit_amount)
    );

    // Step 2: AI operator rebalances funds
    let new_allocations = vec![
        ("helix".to_string(), Decimal::percent(40)),
        ("hydro".to_string(), Decimal::percent(30)),
        ("neptune".to_string(), Decimal::percent(30)),
    ];

    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: new_allocations.clone(),
        reason: "Initial rebalance after deposit".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "rebalance"));

    // Verify protocols have updated allocations
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::GetProtocols {}).unwrap();
    let protocols_info: GetProtocolsResponse = from_json(&query_res).unwrap();

    for protocol in protocols_info.protocols {
        let expected = new_allocations
            .iter()
            .find(|(name, _)| *name == protocol.name)
            .map(|(_, allocation)| *allocation)
            .unwrap_or(Decimal::zero());

        assert_eq!(protocol.allocation_percentage, expected);
    }

    // Step 3: User withdraws half their funds
    let withdrawal_amount = deposit_amount / 2;
    let info = message_info(&user, &[]);
    let msg = ExecuteMsg::Withdraw {
        amount: Uint128::new(withdrawal_amount),
        denom: Some(DENOM.to_string()),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "withdraw"));

    // Verify user info after partial withdrawal
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user.to_string(),
        },
    )
    .unwrap();
    let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();

    // Should be original deposit minus withdrawal (approximately, due to potential fees)
    assert!(user_info.user_info.total_usdc_value.u128() < deposit_amount);
    assert!(user_info.user_info.total_usdc_value.u128() >= deposit_amount - withdrawal_amount - 2); // Allow small rounding difference

    // Step 4: Admin updates a protocol (disables it)
    let info = message_info(&admin, &[]);
    let msg = ExecuteMsg::UpdateProtocol {
        name: "hydro".to_string(),
        enabled: Some(false),
        contract_addr: None,
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "update_protocol"));

    // Verify protocol was disabled
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetProtocolInfo {
            name: "hydro".to_string(),
        },
    )
    .unwrap();
    let protocol_info: GetProtocolInfoResponse = from_json(&query_res).unwrap();

    assert_eq!(protocol_info.protocol.name, "hydro".to_string());
    assert!(!protocol_info.protocol.enabled);

    // Step 5: AI operator rebalances after protocol disable
    let new_allocations = vec![
        ("helix".to_string(), Decimal::percent(50)),
        ("neptune".to_string(), Decimal::percent(50)),
    ];

    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: new_allocations.clone(),
        reason: "Rebalance after disabling hydro".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "rebalance"));

    // Check rebalance history records
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetRebalanceHistory { limit: None },
    )
    .unwrap();
    let history: GetRebalanceHistoryResponse = from_json(&query_res).unwrap();

    assert_eq!(history.history.len(), 2);
    assert_eq!(history.history[0].reason, "Rebalance after disabling hydro");

    // Step 6: User emergency withdraws remaining funds
    let info = message_info(&user, &[]);
    let msg = ExecuteMsg::EmergencyWithdraw {};

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "emergency_withdraw"));

    // User should have no funds left
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user.to_string(),
        },
    )
    .unwrap();
    let user_info: GetUserInfoResponse = from_json(&query_res).unwrap();

    assert_eq!(user_info.user_info.total_usdc_value, Uint128::zero());
}

#[test]
fn test_multi_user_scenario() {
    let (mut deps, _admin, operator, user1) = setup_integration_test();
    let user2 = Addr::unchecked("user2");

    // User 1 deposits
    let deposit_amount1 = 1000u128;
    let info = message_info(&user1, &coins(deposit_amount1, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // User 2 deposits
    let deposit_amount2 = 2000u128;
    let info = message_info(&user2, &coins(deposit_amount2, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // AI operator rebalances
    let allocations = vec![
        ("helix".to_string(), Decimal::percent(30)),
        ("hydro".to_string(), Decimal::percent(30)),
        ("neptune".to_string(), Decimal::percent(40)),
    ];

    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: allocations.clone(),
        reason: "Initial balance".to_string(),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // User 1 withdraws half
    let info = message_info(&user1, &[]);
    let msg = ExecuteMsg::Withdraw {
        amount: Uint128::new(deposit_amount1 / 2),
        denom: Some(DENOM.to_string()),
    };
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Check both users' balances
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user1.to_string(),
        },
    )
    .unwrap();
    let user1_info: GetUserInfoResponse = from_json(&query_res).unwrap();

    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetUserInfo {
            address: user2.to_string(),
        },
    )
    .unwrap();
    let user2_info: GetUserInfoResponse = from_json(&query_res).unwrap();

    // User 1 should have approximately half their deposit left
    assert!(user1_info.user_info.total_usdc_value.u128() <= deposit_amount1 / 2 + 5);
    assert!(user1_info.user_info.total_usdc_value.u128() >= deposit_amount1 / 2 - 5);

    // User 2 should still have their full deposit
    assert_eq!(
        user2_info.user_info.total_usdc_value,
        Uint128::new(deposit_amount2)
    );

    // Both users withdraw all remaining funds
    let info = message_info(&user1, &[]);
    let msg = ExecuteMsg::EmergencyWithdraw {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    let info = message_info(&user2, &[]);
    let msg = ExecuteMsg::EmergencyWithdraw {};
    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Check contract total value should be zero or very close to it
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::GetTotalValue {}).unwrap();
    let total_value_response: GetTotalValueResponse = from_json(&query_res).unwrap();

    // Should be zero or very small due to rounding
    assert!(total_value_response.total_value.u128() < 10);
}
