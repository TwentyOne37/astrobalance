use cosmwasm_std::testing::{
    message_info, mock_dependencies, mock_env, MockApi, MockQuerier, MockStorage,
};
use cosmwasm_std::{coins, from_json, Addr, Decimal, Empty, OwnedDeps};

use crate::contract::{execute, instantiate, query};
use crate::msg::{
    ExecuteMsg, GetProtocolsResponse, GetRebalanceHistoryResponse, InstantiateMsg, QueryMsg,
    RiskParametersMsg,
};
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
fn test_rebalance_invalid_total_allocation() {
    let (mut deps, _admin, operator) = setup_rebalance_test();

    // Try to execute rebalance with allocations that don't sum to 100%
    // 101% exceeds the valid total allocation
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(101)), // Exceeds max 100%
        ],
        reason: "Invalid total allocation".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());
    match res {
        Err(e) => {
            let error_msg = format!("{:?}", e);
            println!("Actual error: {}", error_msg);
            assert!(error_msg.contains("InvalidAllocations"));
        }
        _ => panic!("Expected an error"),
    }
}

#[test]
fn test_rebalance_excessive_allocation() {
    // First update the risk parameters to have a lower max allocation per protocol
    let mut deps = mock_dependencies();
    mock_protocol_response(&mut deps);

    // Setup with custom max allocation per protocol (40%)
    let msg = InstantiateMsg {
        admin: admin_address(),
        ai_operator: operator_address(),
        base_denom: DENOM.to_string(),
        accepted_denoms: vec![DENOM.to_string(), "inj".to_string()],
        astroport_router: router_address(),
        risk_parameters: RiskParametersMsg {
            max_allocation_per_protocol: Decimal::percent(40),
            max_slippage: Decimal::percent(1),
            rebalance_threshold: Decimal::percent(5),
            emergency_withdrawal_fee: Decimal::percent(1),
        },
    };

    let info = message_info(&Addr::unchecked(creator_address()), &[]);
    instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
    setup_test_protocols(&mut deps);

    // Add funds
    let deposit_amount = 1000u128;
    let user = Addr::unchecked(user_address());
    let info = message_info(&user, &coins(deposit_amount, DENOM));
    execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Deposit {}).unwrap();

    // Try to execute rebalance with one protocol exceeding max allocation
    let operator = Addr::unchecked(operator_address());
    let info = message_info(&operator, &[]);
    let msg = ExecuteMsg::Rebalance {
        target_allocations: vec![
            ("helix".to_string(), Decimal::percent(50)), // Exceeds max 40%
            ("hydro".to_string(), Decimal::percent(25)),
            ("neptune".to_string(), Decimal::percent(25)),
        ],
        reason: "Allocation exceeding max per protocol".to_string(),
    };

    let res = execute(deps.as_mut(), mock_env(), info, msg);

    // Should return an error
    assert!(res.is_err());
    match res {
        Err(e) => {
            let error_msg = format!("{:?}", e);
            println!("Actual error: {}", error_msg);
            assert!(error_msg.contains("ExcessiveAllocation"));
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
