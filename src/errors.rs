use quasar_lang::prelude::*;

#[error_code]
pub enum ExternalDelegateTokenMasterError {
    Unauthorized,
    InsufficientVaultFunds,
    MathOverflow,
    InvalidVault,
    InvalidEthAddress,
}
