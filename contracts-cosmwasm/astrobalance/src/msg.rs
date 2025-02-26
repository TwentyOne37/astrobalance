use crate::state::{ProtocolInfo, RebalanceRecord, RiskParameters, UserInfo};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub admin: String,
    pub ai_operator: String,
    pub base_denom: String,           // USDC
    pub accepted_denoms: Vec<String>, // Initial supported tokens
    pub astroport_router: String,     // Astroport router address
    pub risk_parameters: RiskParametersMsg,
}

#[cw_serde]
pub struct RiskParametersMsg {
    pub max_allocation_per_protocol: Decimal,
    pub max_slippage: Decimal,
    pub rebalance_threshold: Decimal,
    pub emergency_withdrawal_fee: Decimal,
}

#[cw_serde]
pub enum ExecuteMsg {
    // User operations
    Deposit {}, // Token info obtained from sent funds
    Withdraw {
        amount: Uint128,
        denom: Option<String>, // If None, withdraw in base_denom (USDC)
    },
    EmergencyWithdraw {},

    // Protocol management
    AddProtocol {
        name: String,
        contract_addr: String,
        initial_allocation: Decimal,
    },
    RemoveProtocol {
        name: String,
    },
    UpdateProtocol {
        name: String,
        enabled: Option<bool>,
        contract_addr: Option<String>,
    },

    // AI Operator functions
    Rebalance {
        target_allocations: Vec<(String, Decimal)>,
        reason: String,
    },
    UpdateRiskParameters {
        risk_parameters: RiskParametersMsg,
    },

    // Admin functions
    AddSupportedToken {
        denom: String,
    },
    RemoveSupportedToken {
        denom: String,
    },
    UpdateAdmin {
        admin: String,
    },
    UpdateAiOperator {
        ai_operator: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetUserInfoResponse)]
    GetUserInfo { address: String },

    #[returns(GetProtocolsResponse)]
    GetProtocols {},

    #[returns(GetProtocolInfoResponse)]
    GetProtocolInfo { name: String },

    #[returns(GetRiskParametersResponse)]
    GetRiskParameters {},

    #[returns(GetRebalanceHistoryResponse)]
    GetRebalanceHistory { limit: Option<u32> },

    #[returns(GetTotalValueResponse)]
    GetTotalValue {},

    #[returns(Config)]
    GetConfig {},
}

#[cw_serde]
pub struct GetUserInfoResponse {
    pub user_info: UserInfo,
}

#[cw_serde]
pub struct GetProtocolsResponse {
    pub protocols: Vec<ProtocolInfo>,
}

#[cw_serde]
pub struct GetProtocolInfoResponse {
    pub protocol: ProtocolInfo,
}

#[cw_serde]
pub struct GetRiskParametersResponse {
    pub risk_parameters: RiskParameters,
}

#[cw_serde]
pub struct GetRebalanceHistoryResponse {
    pub history: Vec<RebalanceRecord>,
}

#[cw_serde]
pub struct GetTotalValueResponse {
    pub total_value: Uint128,
}

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub ai_operator: Addr,
    pub base_denom: String,
    pub accepted_denoms: Vec<String>,
    pub astroport_router: String,
}
