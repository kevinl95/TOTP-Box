use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum CustomContractError {
    #[error("{0}")]
    // let thiserror implement From<StdError> for you
    Std(#[from] StdError),
    #[error("Custom error #1: Cannot add more than one secret.")]
    AlreadyAddedSecret,
}
