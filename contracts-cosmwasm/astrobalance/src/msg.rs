use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub ai_operator: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit { amount: u128 },
    Withdraw { amount: u128 },
    Rebalance {/* Add rebalance parameters as needed */},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetBalanceResponse)]
    GetBalance { address: String },
    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct GetBalanceResponse {
    pub balance: u128,
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub ai_operator: Addr,
}
