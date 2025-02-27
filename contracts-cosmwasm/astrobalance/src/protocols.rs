use crate::error::ContractError;
use cosmwasm_std::{
    to_json_binary, Addr, Coin, CosmosMsg, Decimal, Deps, DepsMut, Env, StdError, StdResult,
    Uint128, WasmMsg,
};

/// Trait defining standard interface for all protocol adapters
pub trait YieldProtocol {
    fn deposit(&self, deps: DepsMut, env: Env, amount: Uint128)
        -> Result<Vec<CosmosMsg>, StdError>;

    fn withdraw(
        &self,
        _deps: DepsMut,
        env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError>;

    fn query_balance(&self, deps: Deps, env: Env) -> StdResult<Uint128>;

    fn query_apy(&self, deps: Deps, env: Env) -> StdResult<Decimal>;

    fn name(&self) -> &str;

    fn protocol_type(&self) -> &str;
}

// Protocol-specific implementations

// Helix Protocol Adapter
pub struct HelixAdapter {
    pub contract_addr: Addr,
    pub name: String,
}

impl YieldProtocol for HelixAdapter {
    fn deposit(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Helix deposit
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&helix::ExecuteMsg::Deposit {})?,
            funds: vec![Coin {
                denom: "usdc".to_string(),
                amount,
            }],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn withdraw(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Helix withdraw
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&helix::ExecuteMsg::Withdraw { amount })?,
            funds: vec![],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn query_balance(&self, deps: Deps, env: Env) -> StdResult<Uint128> {
        // Query balance from Helix
        let balance: helix::BalanceResponse = deps.querier.query_wasm_smart(
            self.contract_addr.to_string(),
            &helix::QueryMsg::Balance {
                address: env.contract.address.to_string(),
            },
        )?;

        Ok(balance.amount)
    }

    fn query_apy(&self, deps: Deps, _env: Env) -> StdResult<Decimal> {
        // Query current APY from Helix
        let apy: helix::ApyResponse = deps
            .querier
            .query_wasm_smart(self.contract_addr.to_string(), &helix::QueryMsg::Apy {})?;

        Ok(apy.apy)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn protocol_type(&self) -> &str {
        "helix"
    }
}

// Hydro Protocol Adapter - Lending & Borrowing
pub struct HydroAdapter {
    pub contract_addr: Addr,
    pub name: String,
}

impl YieldProtocol for HydroAdapter {
    fn deposit(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Hydro deposit - lending pool
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&hydro::ExecuteMsg::SupplyLiquidity {})?,
            funds: vec![Coin {
                denom: "usdc".to_string(),
                amount,
            }],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn withdraw(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Hydro withdraw
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&hydro::ExecuteMsg::WithdrawLiquidity { amount })?,
            funds: vec![],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn query_balance(&self, deps: Deps, env: Env) -> StdResult<Uint128> {
        // Query balance from Hydro
        let balance: hydro::BalanceResponse = deps.querier.query_wasm_smart(
            self.contract_addr.to_string(),
            &hydro::QueryMsg::LenderBalance {
                address: env.contract.address.to_string(),
            },
        )?;

        Ok(balance.supplied_amount)
    }

    fn query_apy(&self, deps: Deps, _env: Env) -> StdResult<Decimal> {
        // Query current APY from Hydro
        let apy: hydro::LendingRateResponse = deps.querier.query_wasm_smart(
            self.contract_addr.to_string(),
            &hydro::QueryMsg::LendingRate {},
        )?;

        Ok(apy.rate)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn protocol_type(&self) -> &str {
        "hydro"
    }
}

// Neptune Finance Adapter - Staking
pub struct NeptuneAdapter {
    pub contract_addr: Addr,
    pub name: String,
}

impl YieldProtocol for NeptuneAdapter {
    fn deposit(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Neptune staking
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&neptune::ExecuteMsg::Stake {})?,
            funds: vec![Coin {
                denom: "usdc".to_string(),
                amount,
            }],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn withdraw(
        &self,
        _deps: DepsMut,
        _env: Env,
        amount: Uint128,
    ) -> Result<Vec<CosmosMsg>, StdError> {
        // Implementation for Neptune unstake
        let msg = WasmMsg::Execute {
            contract_addr: self.contract_addr.to_string(),
            msg: to_json_binary(&neptune::ExecuteMsg::Unstake { amount })?,
            funds: vec![],
        };

        Ok(vec![CosmosMsg::Wasm(msg)])
    }

    fn query_balance(&self, deps: Deps, env: Env) -> StdResult<Uint128> {
        // Query balance from Neptune
        let balance: neptune::StakedBalanceResponse = deps.querier.query_wasm_smart(
            self.contract_addr.to_string(),
            &neptune::QueryMsg::StakedBalance {
                address: env.contract.address.to_string(),
            },
        )?;

        Ok(balance.amount)
    }

    fn query_apy(&self, deps: Deps, _env: Env) -> StdResult<Decimal> {
        // Query current APY from Neptune
        let apy: neptune::StakingRateResponse = deps.querier.query_wasm_smart(
            self.contract_addr.to_string(),
            &neptune::QueryMsg::StakingRate {},
        )?;

        Ok(apy.apy)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn protocol_type(&self) -> &str {
        "neptune"
    }
}

// Protocol interfaces - these would be imported from respective crates in production
pub mod helix {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Decimal, Uint128};

    #[cw_serde]
    pub enum ExecuteMsg {
        Deposit {},
        Withdraw { amount: Uint128 },
    }

    #[cw_serde]
    pub enum QueryMsg {
        Balance { address: String },
        Apy {},
    }

    #[cw_serde]
    pub struct BalanceResponse {
        pub amount: Uint128,
    }

    #[cw_serde]
    pub struct ApyResponse {
        pub apy: Decimal,
    }
}

pub mod hydro {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Decimal, Uint128};

    #[cw_serde]
    pub enum ExecuteMsg {
        SupplyLiquidity {},
        WithdrawLiquidity { amount: Uint128 },
    }

    #[cw_serde]
    pub enum QueryMsg {
        LenderBalance { address: String },
        LendingRate {},
    }

    #[cw_serde]
    pub struct BalanceResponse {
        pub supplied_amount: Uint128,
    }

    #[cw_serde]
    pub struct LendingRateResponse {
        pub rate: Decimal,
    }
}

pub mod neptune {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Decimal, Uint128};

    #[cw_serde]
    pub enum ExecuteMsg {
        Stake {},
        Unstake { amount: Uint128 },
    }

    #[cw_serde]
    pub enum QueryMsg {
        StakedBalance { address: String },
        StakingRate {},
    }

    #[cw_serde]
    pub struct StakedBalanceResponse {
        pub amount: Uint128,
    }

    #[cw_serde]
    pub struct StakingRateResponse {
        pub apy: Decimal,
    }
}

// Factory function to create protocol adapters
pub fn create_protocol_adapter(
    protocol_type: &str,
    contract_addr: Addr,
    name: String,
) -> Result<Box<dyn YieldProtocol>, ContractError> {
    match protocol_type {
        "helix" => Ok(Box::new(HelixAdapter {
            contract_addr,
            name,
        })),
        "hydro" => Ok(Box::new(HydroAdapter {
            contract_addr,
            name,
        })),
        "neptune" => Ok(Box::new(NeptuneAdapter {
            contract_addr,
            name,
        })),
        _ => Err(ContractError::ProtocolNotFound {
            name: protocol_type.to_string(),
        }),
    }
}

// Helper to get all supported protocol types
pub fn get_supported_protocol_types() -> Vec<&'static str> {
    vec!["helix", "hydro", "neptune"]
}
