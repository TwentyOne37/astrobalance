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

    #[error("Multiple denominations not supported")]
    MultipleDenoms {},
}
