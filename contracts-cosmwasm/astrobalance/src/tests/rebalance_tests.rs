use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, Addr, Decimal, Empty, OwnedDeps};

use crate::contract::{execute, query};
use crate::msg::{ExecuteMsg, GetProtocolsResponse, GetRebalanceHistoryResponse, QueryMsg};
use crate::tests::common::*;
use crate::tests::protocol_tests::setup_test_protocols;

// Helper function to set up a test environment with protocols and funds
fn setup_rebalance_test() -> (
    OwnedDeps<MockStorage, MockApi, MockQuerier, Empty>,
    Addr,
    Addr,
) {
    let mut deps = mock_dependencies();
    mock_protocol_response(&mut deps);
    setup_contract(deps.as_mut());
    setup_test_protocols(&mut deps);

    // Add some funds to the contract
    let deposit_amount = 1000u128;
    let user = Addr::unchecked(user_address());
    let info = message_info(&user, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    let admin = Addr::unchecked(admin_address());
    let operator = Addr::unchecked(operator_address());

    (deps, admin, operator)
}

#[test]
fn test_rebalance_basic() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // Define new allocations
    let new_allocations = vec![
        ("helix".to_string(), Decimal::percent(40)),
        ("hydro".to_string(), Decimal::percent(20)),
        ("neptune".to_string(), Decimal::percent(40)),
    ];

    // Execute rebalance as the AI operator
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: new_allocations.clone(),
        reason: "Test rebalance".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Verify response
    assert!(res
        .attributes
        .iter()
        .any(|attr| attr.key == "method" && attr.value == "rebalance"));

    // Query protocols to verify new allocations
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::GetProtocols {}).unwrap();
    let protocols_info: GetProtocolsResponse = from_json(&query_res).unwrap();

    // Verify allocations were updated
    for protocol in protocols_info.protocols {
        let expected = new_allocations
            .iter()
            .find(|(name, _)| *name == protocol.name)
            .map(|(_, allocation)| *allocation)
            .unwrap_or(Decimal::zero());

        assert_eq!(protocol.allocation_percentage, expected);
    }

    // Verify rebalance history was recorded
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetRebalanceHistory { limit: None },
    )
    .unwrap();
    let history: GetRebalanceHistoryResponse = from_json(&query_res).unwrap();

    assert_eq!(history.history.len(), 1);
    assert_eq!(history.history[0].reason, "Test rebalance");
    assert_eq!(history.history[0].initiated_by, operator);
}

#[test]
fn test_rebalance_unauthorized() {
    let (mut deps, _admin, _operator) = setup_rebalance_test();

    // Try to execute rebalance as a regular user (not the AI operator)
    let user = Addr::unchecked(user_address());
    let info = message_info(&user, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(40)),
            ("hydro".to_string(), Decimal::percent(20)),
            ("neptune".to_string(), Decimal::percent(40)),
        ],
        reason: "Unauthorized rebalance".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an unauthorized error
    assert!(res.is_err());
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("Unauthorized"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_rebalance_invalid_allocations() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // Try to execute rebalance with allocations exceeding 100%
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(50)),
            ("hydro".to_string(), Decimal::percent(30)),
            ("neptune".to_string(), Decimal::percent(30)),
        ],
        reason: "Invalid allocations".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("InvalidAllocation"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_rebalance_max_allocation_exceeded() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // Try to execute rebalance with an allocation exceeding the max allowed (50%)
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(60)),
            ("hydro".to_string(), Decimal::percent(20)),
            ("neptune".to_string(), Decimal::percent(20)),
        ],
        reason: "Max allocation exceeded".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("ExcessiveAllocation"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_multiple_rebalances() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // First rebalance
    let first_allocations = vec![
        ("helix".to_string(), Decimal::percent(40)),
        ("hydro".to_string(), Decimal::percent(20)),
        ("neptune".to_string(), Decimal::percent(40)),
    ];

    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: first_allocations.clone(),
        reason: "First rebalance".to_string(),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Second rebalance
    let second_allocations = vec![
        ("helix".to_string(), Decimal::percent(20)),
        ("hydro".to_string(), Decimal::percent(30)),
        ("neptune".to_string(), Decimal::percent(50)),
    ];

    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: second_allocations.clone(),
        reason: "Second rebalance".to_string(),
    };

    execute(deps.as_mut(), mock_env(), info, msg).unwrap();

    // Query rebalance history
    let query_res = query(
        deps.as_ref(),
        mock_env(),
        QueryMsg::GetRebalanceHistory { limit: None },
    )
    .unwrap();
    let history: GetRebalanceHistoryResponse = from_json(&query_res).unwrap();

    // Verify both rebalances were recorded in history
    assert_eq!(history.history.len(), 2);
    assert_eq!(history.history[0].reason, "Second rebalance");
    assert_eq!(history.history[1].reason, "First rebalance");

    // Verify final allocations
    let query_res = query(deps.as_ref(), mock_env(), QueryMsg::GetProtocols {}).unwrap();
    let protocols_info: GetProtocolsResponse = from_json(&query_res).unwrap();

    for protocol in protocols_info.protocols {
        let expected = second_allocations
            .iter()
            .find(|(name, _)| *name == protocol.name)
            .map(|(_, allocation)| *allocation)
            .unwrap_or(Decimal::zero());

        assert_eq!(protocol.allocation_percentage, expected);
    }
}

#[test]
fn test_rebalance_with_nonexistent_protocol() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // Try to execute rebalance with a nonexistent protocol
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(30)),
            ("hydro".to_string(), Decimal::percent(30)),
            ("nonexistent".to_string(), Decimal::percent(40)),
        ],
        reason: "Nonexistent protocol".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());
    match res {
        Err(e) => {
            assert!(format!("{:?}", e).contains("Protocol not found"));
        }
        _ => panic!("Expected an error"),
    }
}
