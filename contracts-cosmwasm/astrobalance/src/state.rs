use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin: Addr,
    pub ai_operator: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const BALANCES: Map<&Addr, u128> = Map::new("balances");
