use anchor_lang::prelude::*;

#[error_code]
pub enum VaultError {
    #[msg("Invalid amount specified")]
    InvalidAmount,
    
    #[msg("Insufficient available balance")]
    InsufficientBalance,
    
    #[msg("Insufficient balance to lock")]
    InsufficientBalanceToLock,
    
    #[msg("Cannot unlock more than locked balance")]
    InvalidUnlockAmount,
    
    #[msg("Unauthorized program attempting to access vault")]
    UnauthorizedProgram,
    
    #[msg("Vault has open positions, cannot withdraw")]
    HasOpenPositions,
    
    #[msg("Overflow in balance calculation")]
    OverflowError,
    
    #[msg("Underflow in balance calculation")]
    UnderflowError,
    
    #[msg("Transfer amount exceeds available balance")]
    InsufficientTransferBalance,
    
    #[msg("Only vault owner can perform this operation")]
    UnauthorizedOwner,
    
    #[msg("Only admin can modify authority")]
    UnauthorizedAdmin,
    
    #[msg("Maximum authorized programs limit reached")]
    MaxAuthorizedProgramsReached,
    
    #[msg("Program not found in authorized list")]
    ProgramNotAuthorized,
    
    #[msg("Invalid vault state")]
    InvalidVaultState,
    
    #[msg("Numerical overflow occurred")]
    NumericalOverflow,
}
