use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid amount")]
    InvalidAmount {},

    #[error("Insufficient funds")]
    InsufficientFunds {},

    #[error("No funds sent")]
    NoFunds {},

    #[error("Invalid denomination. Expected {expected}, received {received}")]
    InvalidDenom { expected: String, received: String },

    #[error("Unsupported denomination: {denom}")]
    UnsupportedDenom { denom: String },

    #[error("Multiple denominations not supported")]
    MultipleDenoms {},

    #[error("Allocation exceeds maximum allowed")]
    ExcessiveAllocation {},

    #[error("Protocol not found: {name}")]
    ProtocolNotFound { name: String },

    #[error("Protocol already exists: {name}")]
    ProtocolAlreadyExists { name: String },

    #[error("Allocations must sum to 100%")]
    InvalidAllocations {},

    #[error("Deposit in progress")]
    DepositInProgress {},

    #[error("Failed to convert token: {error}")]
    ConversionError { error: String },

    #[error("Protocol integration error: {error}")]
    ProtocolError { error: String },

    #[error("Excessive slippage detected")]
    ExcessiveSlippage {},

    #[error("Emergency mode active")]
    EmergencyModeActive {},
}

impl From<ContractError> for StdError {
    fn from(error: ContractError) -> Self {
        StdError::generic_err(error.to_string())
    }
}
