use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    state::{RakuraiActivationAccount, RakuraiActivationConfigAccount},
    ErrorCode::Unauthorized,
};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai Multisig Based Activation Program",
    project_url: "https://rakurai.io/",
    contacts: "https://rakurai.io/company",
    policy: "https://rakurai.io/faq"
}
pub mod sdk;
pub mod state;

declare_id!("pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt");

#[program]
pub mod rakurai_activation {
    use rakurai_vote_state::VoteState;

    use super::*;

    /// Sets up the singleton [RakuraiActivationConfigAccount] to store global configuration settings for Rakurai.
    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        block_builder_authority: Pubkey,
        block_builder_commission_account: Pubkey,
        block_builder_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.block_builder_authority = block_builder_authority;
        cfg.block_builder_commission_account = block_builder_commission_account;
        cfg.block_builder_commission_bps = block_builder_commission_bps;
        cfg.bump = bump;
        cfg.validate()?;

        Ok(())
    }

    /// Update config fields. Only the [RakuraiActivationConfigAccount] authority can invoke this.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_config: RakuraiActivationConfigAccount,
    ) -> Result<()> {
        UpdateConfig::auth(&ctx)?;

        let config = &mut ctx.accounts.config;
        config.authority = new_config.authority;
        config.block_builder_authority = new_config.block_builder_authority;
        config.block_builder_commission_account = new_config.block_builder_commission_account;
        config.block_builder_commission_bps = new_config.block_builder_commission_bps;
        config.validate()?;

        emit!(ConfigUpdatedEvent {
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    /// Initialize a new [RakuraiActivationAccount] associated with the given validator identity account key and seed.
    pub fn initialize_rakurai_activation_account(
        ctx: Context<InitializeRakuraiActivationAccount>,
        validator_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        if ctx.accounts.validator_vote_account.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }

        let validator_vote_state =
            VoteState::deserialize(&ctx.accounts.validator_vote_account).unwrap();
        if validator_vote_state.node_pubkey != ctx.accounts.signer.key() {
            return Err(Unauthorized.into());
        }

        let activation_account = &mut ctx.accounts.activation_account;
        activation_account.is_enabled = false;
        activation_account.hash = None;
        activation_account.proposer = Some(ctx.accounts.signer.key());
        activation_account.validator_commission_bps = validator_commission_bps;
        activation_account.block_builder_commission_bps =
            ctx.accounts.config.block_builder_commission_bps;
        activation_account.validator_authority = ctx.accounts.signer.key();
        activation_account.bump = bump;
        activation_account.validate()?;

        emit!(RakuraiActivationAccountInitializedEvent {
            activation_account: activation_account.key(),
        });

        Ok(())
    }

    /// Updates rakurai activation account approval by either Validator or block builder.
    /// Handles enabling/disabling and hash updates.
    pub fn update_rakurai_activation_approval(
        ctx: Context<UpdateRakuraiActivationApproval>,
        grant_approval: bool,
        hash: Option<[u8; 64]>,
    ) -> Result<()> {
        UpdateRakuraiActivationApproval::auth(&ctx)?;

        let activation_account = &mut ctx.accounts.activation_account;
        let signer_key = ctx.accounts.signer.key();
        let is_block_builder = signer_key == ctx.accounts.config.block_builder_authority;

        if !grant_approval {
            activation_account.is_enabled = false;
            activation_account.hash = None;
            activation_account.proposer = None;
            msg!("Permission Revoked");
        } else if activation_account.is_enabled && is_block_builder {
            activation_account.hash = hash;
            msg!("Hash updated by block builder.");
        } else if !activation_account.is_enabled {
            match activation_account.proposer {
                None => {
                    if is_block_builder {
                        if hash.is_none() {
                            return Err(error!(ErrorCode::MissingHashForEnable));
                        }
                        activation_account.hash = hash;
                        msg!("Proposal initiated by block builder.");
                    } else {
                        msg!("Proposal Pending");
                    }
                    activation_account.proposer = Some(signer_key);
                }
                Some(p) if p == signer_key => {
                    msg!("Proposal Pending");
                }
                Some(_) => {
                    if is_block_builder && hash.is_none() {
                        return Err(error!(ErrorCode::MissingHashForEnable));
                    }
                    if is_block_builder {
                        activation_account.hash = hash;
                    }

                    activation_account.proposer = None;
                    activation_account.is_enabled = true;
                    msg!("Proposal Accepted | Activation enabled");
                }
            }
        }

        activation_account.validate()?;

        emit!(UpdateRakuraiActivationApprovalEvent {
            activation_account: activation_account.key(),
            signer: signer_key,
        });

        Ok(())
    }

    /// Updates commission BPS for validator or block builder based on signer authority.
    pub fn update_rakurai_activation_commission(
        ctx: Context<UpdateRakuraiActivationCommission>,
        commission_bps: u16,
    ) -> Result<()> {
        UpdateRakuraiActivationCommission::auth(&ctx)?;

        let activation_account = &mut ctx.accounts.activation_account;

        if commission_bps > 10_000 {
            return Err(ErrorCode::MaxCommissionBpsExceeded.into());
        }

        if ctx.accounts.signer.key() == activation_account.validator_authority.key() {
            activation_account.validator_commission_bps = commission_bps;
        } else if ctx.accounts.signer.key() == ctx.accounts.config.block_builder_authority.key() {
            activation_account.block_builder_commission_bps = commission_bps;
        } else {
            return Err(Unauthorized.into());
        }
        emit!(UpdateRakuraiActivationCommissionEvent {
            activation_account: activation_account.key(),
            operator_commission: activation_account.validator_commission_bps,
        });

        Ok(())
    }

    /// Closes the Rakurai activation account and claims any remaining lamports.
    pub fn close_rakurai_activation_account(
        ctx: Context<CloseRakuraiActivationAccount>,
    ) -> Result<()> {
        CloseRakuraiActivationAccount::auth(&ctx)?;

        let activation_account = &mut ctx.accounts.activation_account;

        let amount = RakuraiActivationAccount::claim_expired(
            activation_account.to_account_info(),
            ctx.accounts.validator_identity_account.to_account_info(),
        )?;
        emit!(RakuraiActivationAccountClosedEvent {
            activation_account: activation_account.key(),
            amount_claimed: amount,
        });

        Ok(())
    }
}

/// Custom errors for Rakurai activation instructions.
#[error_code]
pub enum ErrorCode {
    #[msg("Account failed validation.")]
    AccountValidationFailure,

    #[msg("Encountered an arithmetic under/overflow error.")]
    ArithmeticError,

    #[msg("Validator's commission basis points must be less than or equal to 10_000")]
    MaxCommissionBpsExceeded,

    #[msg("Hash must be provided when enabling the account as block builder.")]
    MissingHashForEnable,

    #[msg("Unauthorized signer.")]
    Unauthorized,
}

/// Initializes the Rakurai config account with default parameters and stores it at a fixed PDA.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The Rakurai config account (PDA).
    #[account(
        init,
        seeds = [RakuraiActivationConfigAccount::SEED],
        bump,
        payer = initializer,
        space = RakuraiActivationConfigAccount::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// Solana system program required to create accounts.
    pub system_program: Program<'info, System>,

    /// Payer for account creation; must sign the transaction.
    #[account(mut)]
    pub initializer: Signer<'info>,
}

/// Allows the authorized signer to update the Rakurai config parameters.
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// Mutable config account storing Rakurai activation settings.
    #[account(mut, rent_exempt = enforce)]
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// Authorized signer allowed to update the config.
    #[account(mut)]
    pub authority: Signer<'info>,
}

impl UpdateConfig<'_> {
    /// Checks if the signer is the authorized config updater.
    fn auth(ctx: &Context<UpdateConfig>) -> Result<()> {
        if ctx.accounts.config.authority == ctx.accounts.authority.key() {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

/// Initializes a new Rakurai Activation Account(RAA) for a specific validator.
#[derive(Accounts)]
#[instruction(
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRakuraiActivationAccount<'info> {
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// The Rakurai activation PDA account to be created for the validator.
    /// Seeds: [b"rakurai_activation", validator_identity_account.key()]
    #[account(
        init,
        seeds = [
            RakuraiActivationAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump,
        payer = signer,
        space = RakuraiActivationAccount::SIZE,
        rent_exempt = enforce
    )]
    pub activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: The validator's vote account (used for metadata and on-chain validation).
    pub validator_vote_account: AccountInfo<'info>,

    /// CHECK: The validator's identity account (used to derive the PDA and verify authority).
    pub validator_identity_account: AccountInfo<'info>,

    /// Payer for account creation; must sign the transaction. In current context validator's identity account.
    #[account(mut)]
    pub signer: Signer<'info>,

    /// Standard Solana system program for account creation.
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRakuraiActivationApproval<'info> {
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// PDA storing validator-specific Rakurai activation state.
    #[account(
        mut,
        seeds = [
            RakuraiActivationAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = activation_account.bump,
        rent_exempt = enforce,
    )]
    pub activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: Validator identity associated with the activation account
    pub validator_identity_account: AccountInfo<'info>,

    /// Signer must match either validator authority or block builder authority
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateRakuraiActivationApproval<'_> {
    /// Authorizes signer as either validator authority or block builder authority
    fn auth(ctx: &Context<UpdateRakuraiActivationApproval>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.activation_account.validator_authority.key()
            || ctx.accounts.signer.key() == ctx.accounts.config.block_builder_authority.key()
        {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

/// Updates the Rakurai activation commission for a specific validator.
#[derive(Accounts)]
pub struct UpdateRakuraiActivationCommission<'info> {
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// PDA storing validator-specific Rakurai activation state.
    #[account(
        mut,
        seeds = [
            RakuraiActivationAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = activation_account.bump,
        rent_exempt = enforce,
    )]
    pub activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: Validator identity associated with the activation account.
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,

    /// Signer who must be either validator authority or block builder authority.
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateRakuraiActivationCommission<'_> {
    /// Checks if signer is authorized to update commission (validator or block builder authority).
    fn auth(ctx: &Context<UpdateRakuraiActivationCommission>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.activation_account.validator_authority.key()
            || ctx.accounts.signer.key() == ctx.accounts.config.block_builder_authority.key()
        {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

#[derive(Accounts)]
pub struct CloseRakuraiActivationAccount<'info> {
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    /// PDA storing validator-specific Rakurai activation state; closed during this instruction.
    #[account(
        mut,
        close = validator_identity_account,
        seeds = [
            RakuraiActivationAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = activation_account.bump,
        rent_exempt = enforce,
    )]
    pub activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: Validator's identity account that receives the closed account's lamports.
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,

    /// Signer authorized to close activation accounts (must match block_builder_authority).
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseRakuraiActivationAccount<'_> {
    /// Ensures the signer is the `block_builder_authority` from the config.
    fn auth(ctx: &Context<CloseRakuraiActivationAccount>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.config.block_builder_authority.key() {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

// Events

/// Emitted when the global config is updated.
#[event]
pub struct ConfigUpdatedEvent {
    /// The authority that performed the update.
    authority: Pubkey,
}

/// Emitted when a new Rakurai activation account is initialized.
#[event]
pub struct RakuraiActivationAccountInitializedEvent {
    /// The newly initialized activation account.
    pub activation_account: Pubkey,
}

/// Emitted when one of the authorities approves a Rakurai activation update.
#[event]
pub struct UpdateRakuraiActivationApprovalEvent {
    /// The activation account receiving the update.
    pub activation_account: Pubkey,
    /// The signer (authority) who approved the update.
    pub signer: Pubkey,
}

/// Emitted when the operator commission of an activation account is updated.
#[event]
pub struct UpdateRakuraiActivationCommissionEvent {
    /// The activation account whose commission was updated.
    pub activation_account: Pubkey,
    /// The new operator commission (basis points or percent, depending on program logic).
    pub operator_commission: u16,
}

/// Emitted when a Rakurai activation account is closed and funds are claimed.
#[event]
pub struct RakuraiActivationAccountClosedEvent {
    /// The closed activation account.
    pub activation_account: Pubkey,
    /// Total lamports claimed during closure.
    pub amount_claimed: u64,
}
