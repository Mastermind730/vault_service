use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};
use anchor_spl::associated_token::AssociatedToken;

pub mod state;
pub mod errors;

pub use state::*;
pub use errors::*;

declare_id!("VAULTmngr11111111111111111111111111111111");

#[program]
pub mod vault_manager {
    use super::*;

    /// Initialize a new collateral vault for a user
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?;
        
        vault.owner = ctx.accounts.user.key();
        vault.token_account = ctx.accounts.vault_token_account.key();
        vault.total_balance = 0;
        vault.locked_balance = 0;
        vault.available_balance = 0;
        vault.total_deposited = 0;
        vault.total_withdrawn = 0;
        vault.created_at = clock.unix_timestamp;
        vault.last_updated = clock.unix_timestamp;
        vault.bump = ctx.bumps.vault;
        
        msg!("Vault initialized for user: {}", ctx.accounts.user.key());
        Ok(())
    }

    /// Initialize the vault authority (one-time setup)
    pub fn initialize_authority(ctx: Context<InitializeAuthority>) -> Result<()> {
        let authority = &mut ctx.accounts.authority;
        
        authority.authorized_programs = Vec::new();
        authority.admin = ctx.accounts.admin.key();
        authority.bump = ctx.bumps.authority;
        
        msg!("Vault authority initialized");
        Ok(())
    }

    /// Add an authorized program that can lock/unlock collateral
    pub fn add_authorized_program(
        ctx: Context<ManageAuthority>,
        program_id: Pubkey,
    ) -> Result<()> {
        let authority = &mut ctx.accounts.authority;
        
        require!(
            authority.authorized_programs.len() < VaultAuthority::MAX_AUTHORIZED_PROGRAMS,
            VaultError::MaxAuthorizedProgramsReached
        );
        
        if !authority.authorized_programs.contains(&program_id) {
            authority.authorized_programs.push(program_id);
        }
        
        emit!(AuthorityUpdatedEvent {
            authority: authority.key(),
            program: program_id,
            authorized: true,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        msg!("Added authorized program: {}", program_id);
        Ok(())
    }

    /// Remove an authorized program
    pub fn remove_authorized_program(
        ctx: Context<ManageAuthority>,
        program_id: Pubkey,
    ) -> Result<()> {
        let authority = &mut ctx.accounts.authority;
        
        authority.authorized_programs.retain(|&x| x != program_id);
        
        emit!(AuthorityUpdatedEvent {
            authority: authority.key(),
            program: program_id,
            authorized: false,
            timestamp: Clock::get()?.unix_timestamp,
        });
        
        msg!("Removed authorized program: {}", program_id);
        Ok(())
    }

    /// Deposit USDT collateral into the vault
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        
        let clock = Clock::get()?;
        
        // Transfer USDT from user to vault using CPI
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.user_token_account.to_account_info(),
                    to: ctx.accounts.vault_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        // Update vault state
        let vault = &mut ctx.accounts.vault;
        vault.total_balance = vault.total_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.available_balance = vault.available_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.total_deposited = vault.total_deposited
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.last_updated = clock.unix_timestamp;

        emit!(DepositEvent {
            user: ctx.accounts.user.key(),
            vault: vault.key(),
            amount,
            new_balance: vault.total_balance,
            timestamp: clock.unix_timestamp,
        });

        msg!("Deposited {} tokens to vault", amount);
        Ok(())
    }

    /// Withdraw USDT collateral from the vault
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?;
        
        // Verify sufficient available balance
        require!(
            vault.available_balance >= amount,
            VaultError::InsufficientBalance
        );
        
        // Verify no locked collateral preventing withdrawal
        require!(
            vault.locked_balance == 0 || vault.available_balance >= amount,
            VaultError::HasOpenPositions
        );

        // Transfer USDT from vault to user using CPI with PDA signer
        let user_key = ctx.accounts.user.key();
        let seeds = &[
            b"vault",
            user_key.as_ref(),
            &[vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault_token_account.to_account_info(),
                    to: ctx.accounts.user_token_account.to_account_info(),
                    authority: vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        // Update vault state
        vault.total_balance = vault.total_balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;
        vault.available_balance = vault.available_balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;
        vault.total_withdrawn = vault.total_withdrawn
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.last_updated = clock.unix_timestamp;

        emit!(WithdrawalEvent {
            user: ctx.accounts.user.key(),
            vault: vault.key(),
            amount,
            new_balance: vault.total_balance,
            timestamp: clock.unix_timestamp,
        });

        msg!("Withdrawn {} tokens from vault", amount);
        Ok(())
    }

    /// Lock collateral for margin requirements (called by authorized programs via CPI)
    pub fn lock_collateral(ctx: Context<LockCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?;
        
        // Verify sufficient available balance
        require!(
            vault.available_balance >= amount,
            VaultError::InsufficientBalanceToLock
        );

        // Update balances
        vault.locked_balance = vault.locked_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.available_balance = vault.available_balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;
        vault.last_updated = clock.unix_timestamp;

        emit!(LockEvent {
            vault: vault.key(),
            amount,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            timestamp: clock.unix_timestamp,
        });

        msg!("Locked {} tokens in vault", amount);
        Ok(())
    }

    /// Unlock collateral when position is closed (called by authorized programs via CPI)
    pub fn unlock_collateral(ctx: Context<UnlockCollateral>, amount: u64) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        
        let vault = &mut ctx.accounts.vault;
        let clock = Clock::get()?;
        
        // Verify sufficient locked balance
        require!(
            vault.locked_balance >= amount,
            VaultError::InvalidUnlockAmount
        );

        // Update balances
        vault.locked_balance = vault.locked_balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;
        vault.available_balance = vault.available_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        vault.last_updated = clock.unix_timestamp;

        emit!(UnlockEvent {
            vault: vault.key(),
            amount,
            locked_balance: vault.locked_balance,
            available_balance: vault.available_balance,
            timestamp: clock.unix_timestamp,
        });

        msg!("Unlocked {} tokens in vault", amount);
        Ok(())
    }

    /// Transfer collateral between vaults (for settlements/liquidations)
    pub fn transfer_collateral(
        ctx: Context<TransferCollateral>,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, VaultError::InvalidAmount);
        
        let from_vault = &mut ctx.accounts.from_vault;
        let to_vault = &mut ctx.accounts.to_vault;
        let clock = Clock::get()?;
        
        // Verify sufficient balance in source vault
        require!(
            from_vault.total_balance >= amount,
            VaultError::InsufficientTransferBalance
        );

        // Update source vault
        from_vault.total_balance = from_vault.total_balance
            .checked_sub(amount)
            .ok_or(VaultError::UnderflowError)?;
        from_vault.last_updated = clock.unix_timestamp;

        // Update destination vault
        to_vault.total_balance = to_vault.total_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        to_vault.available_balance = to_vault.available_balance
            .checked_add(amount)
            .ok_or(VaultError::NumericalOverflow)?;
        to_vault.last_updated = clock.unix_timestamp;

        // Transfer tokens between vault token accounts
        let from_owner_key = from_vault.owner.key();
        let seeds = &[
            b"vault",
            from_owner_key.as_ref(),
            &[from_vault.bump],
        ];
        let signer_seeds = &[&seeds[..]];

        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.from_token_account.to_account_info(),
                    to: ctx.accounts.to_token_account.to_account_info(),
                    authority: from_vault.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        emit!(TransferEvent {
            from_vault: from_vault.key(),
            to_vault: to_vault.key(),
            amount,
            timestamp: clock.unix_timestamp,
        });

        msg!("Transferred {} tokens between vaults", amount);
        Ok(())
    }
}

// ============ Account Validation Contexts ============

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        init,
        payer = user,
        space = CollateralVault::LEN,
        seeds = [b"vault", user.key().as_ref()],
        bump
    )]
    pub vault: Account<'info, CollateralVault>,

    #[account(
        init,
        payer = user,
        associated_token::mint = mint,
        associated_token::authority = vault,
    )]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = VaultAuthority::LEN,
        seeds = [b"authority"],
        bump
    )]
    pub authority: Account<'info, VaultAuthority>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageAuthority<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"authority"],
        bump = authority.bump,
        has_one = admin @ VaultError::UnauthorizedAdmin
    )]
    pub authority: Account<'info, VaultAuthority>,
}

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub vault: Account<'info, CollateralVault>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub owner: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        seeds = [b"vault", user.key().as_ref()],
        bump = vault.bump,
        has_one = owner @ VaultError::UnauthorizedOwner
    )]
    pub vault: Account<'info, CollateralVault>,

    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub vault_token_account: Account<'info, TokenAccount>,

    pub owner: SystemAccount<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct LockCollateral<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.owner.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, CollateralVault>,

    #[account(
        seeds = [b"authority"],
        bump = authority.bump,
    )]
    pub authority: Account<'info, VaultAuthority>,
}

#[derive(Accounts)]
pub struct UnlockCollateral<'info> {
    #[account(
        mut,
        seeds = [b"vault", vault.owner.as_ref()],
        bump = vault.bump,
    )]
    pub vault: Account<'info, CollateralVault>,

    #[account(
        seeds = [b"authority"],
        bump = authority.bump,
    )]
    pub authority: Account<'info, VaultAuthority>,
}

#[derive(Accounts)]
pub struct TransferCollateral<'info> {
    #[account(
        mut,
        seeds = [b"vault", from_vault.owner.as_ref()],
        bump = from_vault.bump,
    )]
    pub from_vault: Account<'info, CollateralVault>,

    #[account(
        mut,
        seeds = [b"vault", to_vault.owner.as_ref()],
        bump = to_vault.bump,
    )]
    pub to_vault: Account<'info, CollateralVault>,

    #[account(mut)]
    pub from_token_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub to_token_account: Account<'info, TokenAccount>,

    #[account(
        seeds = [b"authority"],
        bump = authority.bump,
    )]
    pub authority: Account<'info, VaultAuthority>,

    pub token_program: Program<'info, Token>,
}
