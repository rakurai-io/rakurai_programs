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

    /// Initialize a singleton instance of the [RakuraiActivationConfigAccount] account.
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

    /// Initialize a new [RakuraiActivationAccount] associated with the given validator vote key
    /// and current epoch.
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

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [RakuraiActivationConfigAccount::SEED],
        bump,
        payer = initializer,
        space = RakuraiActivationConfigAccount::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, RakuraiActivationConfigAccount>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub initializer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, rent_exempt = enforce)]
    pub config: Account<'info, RakuraiActivationConfigAccount>,
    #[account(mut)]
    pub authority: Signer<'info>,
}

impl UpdateConfig<'_> {
    fn auth(ctx: &Context<UpdateConfig>) -> Result<()> {
        if ctx.accounts.config.authority == ctx.accounts.authority.key() {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

#[derive(Accounts)]
#[instruction(
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRakuraiActivationAccount<'info> {
    pub config: Account<'info, RakuraiActivationConfigAccount>,
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
    pub validator_vote_account: AccountInfo<'info>,
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateRakuraiActivationApproval<'info> {
    pub config: Account<'info, RakuraiActivationConfigAccount>,
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
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateRakuraiActivationApproval<'_> {
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

#[derive(Accounts)]
pub struct UpdateRakuraiActivationCommission<'info> {
    pub config: Account<'info, RakuraiActivationConfigAccount>,
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
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateRakuraiActivationCommission<'_> {
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
    pub config: Account<'info, RakuraiActivationConfigAccount>,
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
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseRakuraiActivationAccount<'_> {
    fn auth(ctx: &Context<CloseRakuraiActivationAccount>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.config.block_builder_authority.key() {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

// Events

#[event]
pub struct ConfigUpdatedEvent {
    /// Who updated it.
    authority: Pubkey,
}

#[event]
pub struct RakuraiActivationAccountInitializedEvent {
    pub activation_account: Pubkey,
}

#[event]
pub struct UpdateRakuraiActivationApprovalEvent {
    pub activation_account: Pubkey,
    pub signer: Pubkey,
}

#[event]
pub struct UpdateRakuraiActivationCommissionEvent {
    pub activation_account: Pubkey,
    pub operator_commission: u16,
}

#[event]
pub struct RakuraiActivationAccountClosedEvent {
    pub activation_account: Pubkey,
    pub amount_claimed: u64,
}
