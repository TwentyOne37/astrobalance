use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Decimal, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub ai_operator: Addr,
    pub base_denom: String,           // USDC - our standard denomination
    pub accepted_denoms: Vec<String>, // List of supported tokens
    pub astroport_router: String,     // Added to match with msg::Config
}

#[cw_serde]
pub struct UserDeposit {
    pub original_token: String,
    pub original_amount: Uint128,
    pub usdc_value_at_deposit: Uint128,
    pub timestamp: Timestamp,
}

#[cw_serde]
pub struct UserInfo {
    pub total_usdc_value: Uint128,
    pub deposits: Vec<UserDeposit>,
}

#[cw_serde]
pub struct ProtocolInfo {
    pub name: String,
    pub contract_addr: Addr,
    pub allocation_percentage: Decimal, // Current allocation percentage
    pub current_balance: Uint128,       // Current USDC value in this protocol
    pub enabled: bool,
}

#[cw_serde]
pub struct RiskParameters {
    pub max_allocation_per_protocol: Decimal, // Max % in any single protocol
    pub max_slippage: Decimal,                // Max slippage for swaps
    pub rebalance_threshold: Decimal,         // Min difference to trigger rebalance
    pub emergency_withdrawal_fee: Decimal,    // Fee for emergency withdrawals
}

#[cw_serde]
pub struct RebalanceRecord {
    pub timestamp: Timestamp,
    pub initiated_by: Addr,
    pub old_allocations: Vec<(String, Decimal)>,
    pub new_allocations: Vec<(String, Decimal)>,
    pub reason: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const USER_INFOS: Map<&Addr, UserInfo> = Map::new("user_infos");
pub const PROTOCOLS: Map<&String, ProtocolInfo> = Map::new("protocols");
pub const RISK_PARAMETERS: Item<RiskParameters> = Item::new("risk_parameters");
pub const REBALANCE_HISTORY: Item<Vec<RebalanceRecord>> = Item::new("rebalance_history");
pub const TOTAL_USDC_VALUE: Item<Uint128> = Item::new("total_usdc_value");
