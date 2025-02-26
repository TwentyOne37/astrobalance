use cosmwasm_std::{
    Addr, Decimal, Deps, DepsMut, Env, Fraction, MessageInfo, Response, StdError, StdResult,
    Storage, Uint128,
};
use std::collections::HashMap;

use crate::error::ContractError;
use crate::protocols::create_protocol_adapter;
use crate::state::{ProtocolInfo, RebalanceRecord, PROTOCOLS, REBALANCE_HISTORY, TOTAL_USDC_VALUE};

pub struct StrategyExecutor {}

impl StrategyExecutor {
    // Calculate and validate target allocations
    pub fn validate_allocations(
        target_allocations: &[(String, Decimal)],
        max_per_protocol: Decimal,
    ) -> Result<(), ContractError> {
        // Check that allocations sum to 100%
        let total_allocation: Decimal = target_allocations.iter().map(|(_, alloc)| *alloc).sum();

        if total_allocation != Decimal::one() {
            return Err(ContractError::InvalidAllocations {});
        }

        // Check that no protocol exceeds maximum allocation
        for (_, allocation) in target_allocations {
            if *allocation > max_per_protocol {
                return Err(ContractError::ExcessiveAllocation {});
            }
        }

        Ok(())
    }

    // Calculate actions needed to achieve target allocations
    pub fn calculate_rebalance_actions(
        deps: Deps,
        current_protocols: Vec<ProtocolInfo>,
        target_allocations: &[(String, Decimal)],
        total_value: Uint128,
    ) -> StdResult<RebalanceActions> {
        let mut withdrawals = vec![];
        let mut deposits = vec![];

        // Create maps for easier lookup
        let mut current_map: HashMap<String, (Decimal, Uint128)> = HashMap::new();
        for protocol in current_protocols {
            current_map.insert(
                protocol.name.clone(),
                (protocol.allocation_percentage, protocol.current_balance),
            );
        }

        let mut target_map: HashMap<String, Decimal> = HashMap::new();
        for (name, allocation) in target_allocations {
            target_map.insert(name.clone(), *allocation);
        }

        // Calculate withdrawals (protocols that need reduction)
        for (name, (current_percentage, current_balance)) in &current_map {
            let zero_decimal = Decimal::zero();
            let target_percentage = target_map.get(name).unwrap_or(&zero_decimal);

            if target_percentage < current_percentage {
                // This protocol needs reduction
                let target_balance = total_value.multiply_ratio(
                    target_percentage.numerator(),
                    target_percentage.denominator(),
                );
                let withdrawal_amount = current_balance.saturating_sub(target_balance);

                if !withdrawal_amount.is_zero() {
                    // Retrieve protocol info to get contract address
                    if let Ok(Some(protocol_info)) = PROTOCOLS.may_load(deps.storage, name) {
                        withdrawals.push(RebalanceAction {
                            protocol_name: name.clone(),
                            contract_addr: protocol_info.contract_addr,
                            amount: withdrawal_amount,
                        });
                    }
                }
            }
        }

        // Calculate deposits (protocols that need increase)
        for (name, target_percentage) in &target_map {
            let zero_tuple = (Decimal::zero(), Uint128::zero());
            let (current_percentage, _) = current_map.get(name).unwrap_or(&zero_tuple);

            if target_percentage > current_percentage {
                // This protocol needs increase
                let target_balance = total_value.multiply_ratio(
                    target_percentage.numerator(),
                    target_percentage.denominator(),
                );
                let current_balance = current_map
                    .get(name)
                    .map(|(_, balance)| *balance)
                    .unwrap_or(Uint128::zero());

                let deposit_amount = target_balance.saturating_sub(current_balance);

                if !deposit_amount.is_zero() {
                    // Retrieve protocol info to get contract address
                    if let Ok(Some(protocol_info)) = PROTOCOLS.may_load(deps.storage, name) {
                        deposits.push(RebalanceAction {
                            protocol_name: name.clone(),
                            contract_addr: protocol_info.contract_addr,
                            amount: deposit_amount,
                        });
                    }
                }
            }
        }

        Ok(RebalanceActions {
            withdrawals,
            deposits,
        })
    }

    // Execute rebalancing using the calculated actions
    pub fn execute_rebalance(
        mut deps: DepsMut,
        env: Env,
        info: MessageInfo,
        target_allocations: Vec<(String, Decimal)>,
        reason: String,
        max_allocation_per_protocol: Decimal,
    ) -> Result<Response, ContractError> {
        // Validate allocations first
        Self::validate_allocations(&target_allocations, max_allocation_per_protocol)?;

        // Load current protocol data
        let mut current_protocols = vec![];
        let protocol_names: Vec<String> = PROTOCOLS
            .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|key| key.unwrap())
            .collect();

        for name in protocol_names {
            if let Some(protocol) = PROTOCOLS.may_load(deps.storage, &name)? {
                current_protocols.push(protocol);
            }
        }

        // Save old allocations for history
        let old_allocations: Vec<(String, Decimal)> = current_protocols
            .iter()
            .map(|p| (p.name.clone(), p.allocation_percentage))
            .collect();

        // Get total value
        let total_value = TOTAL_USDC_VALUE.load(deps.storage)?;

        // Calculate actions needed
        let actions = Self::calculate_rebalance_actions(
            deps.as_ref(),
            current_protocols,
            &target_allocations,
            total_value,
        )?;

        // Start building messages and response
        let mut messages = vec![];

        // First execute all withdrawals
        for action in &actions.withdrawals {
            let protocol_adapter = create_protocol_adapter(
                &action.protocol_name,
                action.contract_addr.clone(),
                action.protocol_name.clone(),
            )?;

            let withdraw_msgs =
                protocol_adapter.withdraw(deps.branch(), env.clone(), action.amount)?;
            messages.extend(withdraw_msgs);
        }

        // Then execute all deposits
        for action in &actions.deposits {
            let protocol_adapter = create_protocol_adapter(
                &action.protocol_name,
                action.contract_addr.clone(),
                action.protocol_name.clone(),
            )?;

            let deposit_msgs =
                protocol_adapter.deposit(deps.branch(), env.clone(), action.amount)?;
            messages.extend(deposit_msgs);
        }

        // Update protocol allocations and balances
        for (name, new_allocation) in &target_allocations {
            PROTOCOLS.update(deps.storage, name, |protocol_opt| -> StdResult<_> {
                let mut protocol = protocol_opt.ok_or_else(|| {
                    StdError::generic_err(format!("Protocol not found: {}", name))
                })?;

                protocol.allocation_percentage = *new_allocation;
                // The actual balance will be updated in the next query cycle

                Ok(protocol)
            })?;
        }

        // Record rebalance history
        record_rebalance(
            deps.storage,
            env.block.time,
            info.sender,
            old_allocations,
            target_allocations.clone(),
            reason.clone(),
        )?;

        Ok(Response::new()
            .add_messages(messages)
            .add_attribute("method", "rebalance")
            .add_attribute("reason", reason)
            .add_attribute("withdrawals", actions.withdrawals.len().to_string())
            .add_attribute("deposits", actions.deposits.len().to_string()))
    }

    // Check if rebalance is needed based on the threshold
    pub fn check_rebalance_needed(
        deps: Deps,
        target_allocations: &[(String, Decimal)],
        rebalance_threshold: Decimal,
    ) -> StdResult<bool> {
        // Get current protocols and allocations
        let protocol_names: Vec<String> = PROTOCOLS
            .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|key| key.unwrap())
            .collect();

        let mut current_allocations = HashMap::new();
        for name in protocol_names {
            if let Some(protocol) = PROTOCOLS.may_load(deps.storage, &name)? {
                current_allocations.insert(protocol.name, protocol.allocation_percentage);
            }
        }

        // Check if any allocation deviates more than the threshold
        for (name, target_alloc) in target_allocations {
            let zero_decimal = Decimal::zero();
            let current_alloc = current_allocations.get(name).unwrap_or(&zero_decimal);
            let difference = if *target_alloc > *current_alloc {
                *target_alloc - *current_alloc
            } else {
                *current_alloc - *target_alloc
            };

            if difference > rebalance_threshold {
                return Ok(true);
            }
        }

        Ok(false)
    }

    // Update protocol balances by querying each protocol
    pub fn update_protocol_balances(deps: DepsMut, env: Env) -> Result<(), ContractError> {
        let mut total_balance = Uint128::zero();
        let protocol_names: Vec<String> = PROTOCOLS
            .keys(deps.storage, None, None, cosmwasm_std::Order::Ascending)
            .map(|key| key.unwrap())
            .collect();

        // First collect all balances to avoid the borrow conflict
        let mut balances = HashMap::new();

        for name in &protocol_names {
            let protocol_info = PROTOCOLS.load(deps.storage, name)?;
            let protocol_adapter =
                create_protocol_adapter(&name, protocol_info.contract_addr.clone(), name.clone())?;

            let current_balance = protocol_adapter.query_balance(deps.as_ref(), env.clone())?;
            balances.insert(name.clone(), current_balance);
            total_balance += current_balance;
        }

        // Now update all protocols with their balances
        for name in protocol_names {
            if let Some(balance) = balances.get(&name) {
                PROTOCOLS.update(deps.storage, &name, |maybe_protocol| -> StdResult<_> {
                    let mut protocol = maybe_protocol.ok_or_else(|| {
                        StdError::generic_err(format!("Protocol not found: {}", name))
                    })?;

                    protocol.current_balance = *balance;
                    Ok(protocol)
                })?;
            }
        }

        // Update total USDC value
        TOTAL_USDC_VALUE.save(deps.storage, &total_balance)?;

        Ok(())
    }
}

// Structure to track rebalance actions
pub struct RebalanceAction {
    pub protocol_name: String,
    pub contract_addr: Addr,
    pub amount: Uint128,
}

pub struct RebalanceActions {
    pub withdrawals: Vec<RebalanceAction>,
    pub deposits: Vec<RebalanceAction>,
}

// Helper to record rebalance history
pub fn record_rebalance(
    storage: &mut dyn Storage,
    timestamp: cosmwasm_std::Timestamp,
    initiated_by: Addr,
    old_allocations: Vec<(String, Decimal)>,
    new_allocations: Vec<(String, Decimal)>,
    reason: String,
) -> StdResult<Vec<RebalanceRecord>> {
    REBALANCE_HISTORY.update(storage, |mut history| -> StdResult<_> {
        history.push(RebalanceRecord {
            timestamp,
            initiated_by,
            old_allocations,
            new_allocations,
            reason,
        });

        // Limit history size to prevent excessive storage growth
        if history.len() > 20 {
            let len = history.len();
            history = history.drain(0..(len - 20)).collect();
        }

        Ok(history)
    })
}
