use anchor_lang::prelude::*;

/// Main vault account that holds user collateral information
#[account]
pub struct CollateralVault {
    /// Owner of the vault (user's public key)
    pub owner: Pubkey,
    
    /// Associated token account that holds the actual USDT
    pub token_account: Pubkey,
    
    /// Total balance in the vault (locked + available)
    pub total_balance: u64,
    
    /// Balance locked for open positions
    pub locked_balance: u64,
    
    /// Available balance for withdrawal (total - locked)
    pub available_balance: u64,
    
    /// Cumulative amount deposited over lifetime
    pub total_deposited: u64,
    
    /// Cumulative amount withdrawn over lifetime
    pub total_withdrawn: u64,
    
    /// Timestamp when vault was created
    pub created_at: i64,
    
    /// Last activity timestamp
    pub last_updated: i64,
    
    /// PDA bump seed
    pub bump: u8,
}

impl CollateralVault {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        32 + // token_account
        8 +  // total_balance
        8 +  // locked_balance
        8 +  // available_balance
        8 +  // total_deposited
        8 +  // total_withdrawn
        8 +  // created_at
        8 +  // last_updated
        1;   // bump
}

/// Authority account that manages authorized programs
#[account]
pub struct VaultAuthority {
    /// Programs authorized to lock/unlock collateral
    pub authorized_programs: Vec<Pubkey>,
    
    /// Admin who can add/remove authorized programs
    pub admin: Pubkey,
    
    /// PDA bump seed
    pub bump: u8,
}

impl VaultAuthority {
    pub const MAX_AUTHORIZED_PROGRAMS: usize = 10;
    
    pub const LEN: usize = 8 + // discriminator
        4 + (32 * Self::MAX_AUTHORIZED_PROGRAMS) + // authorized_programs vector
        32 + // admin
        1;   // bump
}

/// Transaction types supported by the vault
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    Lock,
    Unlock,
    Transfer,
}

/// Event emitted when a deposit occurs
#[event]
pub struct DepositEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when a withdrawal occurs
#[event]
pub struct WithdrawalEvent {
    pub user: Pubkey,
    pub vault: Pubkey,
    pub amount: u64,
    pub new_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when collateral is locked
#[event]
pub struct LockEvent {
    pub vault: Pubkey,
    pub amount: u64,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when collateral is unlocked
#[event]
pub struct UnlockEvent {
    pub vault: Pubkey,
    pub amount: u64,
    pub locked_balance: u64,
    pub available_balance: u64,
    pub timestamp: i64,
}

/// Event emitted when collateral is transferred between vaults
#[event]
pub struct TransferEvent {
    pub from_vault: Pubkey,
    pub to_vault: Pubkey,
    pub amount: u64,
    pub timestamp: i64,
}

/// Event emitted when vault authority is updated
#[event]
pub struct AuthorityUpdatedEvent {
    pub authority: Pubkey,
    pub program: Pubkey,
    pub authorized: bool,
    pub timestamp: i64,
}
