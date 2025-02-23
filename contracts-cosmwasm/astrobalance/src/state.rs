use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub ai_operator: Addr,
    pub accepted_denom: String, // Which native token we accept
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const BALANCES: Map<&Addr, u128> = Map::new("balances");
