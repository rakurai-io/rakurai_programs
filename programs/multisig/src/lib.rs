use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use {default_env::default_env, solana_security_txt::security_txt};

use crate::{
    state::{Config, MultiSigAccount},
    ErrorCode::Unauthorized,
};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai MultiSig Program",
    project_url: "https://rakurai.io/",
    contacts: "https://rakurai.io/company",
    policy: "https://rakurai.io/faq",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/rakurai-io/rakurai_programs",
    source_revision: default_env!("GIT_SHA", "GIT_SHA_MISSING"),
    source_release: default_env!("GIT_REF_NAME", "GIT_REF_NAME_MISSING")
}
pub mod sdk;
pub mod state;

declare_id!("2Q7DK4qWRAQvYNseZ3UnWLQYjZFgyRJurP7NJDvDCusF");

#[program]
pub mod multi_sig {
    use rakurai_vote_state::VoteState;

    use super::*;

    /// Initialize a singleton instance of the [Config] account.
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

    /// Update config fields. Only the [Config] authority can invoke this.
    pub fn update_config(ctx: Context<UpdateConfig>, new_config: Config) -> Result<()> {
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

    /// Initialize a new [MultiSigAccount] associated with the given validator vote key
    /// and current epoch.
    pub fn initialize_multi_sig_account(
        ctx: Context<InitializeMultiSigAccount>,
        validator_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        if ctx.accounts.validator_vote_account.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }

        let validator_vote_state = VoteState::deserialize(&ctx.accounts.validator_vote_account)?;
        if &validator_vote_state.node_pubkey != ctx.accounts.signer.key {
            return Err(Unauthorized.into());
        }

        let multisig_account = &mut ctx.accounts.multisig_account;
        multisig_account.is_enabled = false;
        multisig_account.proposer = Some(ctx.accounts.signer.key());
        multisig_account.validator_commission_bps = validator_commission_bps;
        multisig_account.validator_vote_account = ctx.accounts.validator_vote_account.key();
        multisig_account.block_builder_commission_bps =
            ctx.accounts.config.block_builder_commission_bps;
        multisig_account.block_builder_commission_account =
            ctx.accounts.config.block_builder_commission_account;
        multisig_account.validator_authority = ctx.accounts.signer.key();
        multisig_account.block_builder_authority = ctx.accounts.config.block_builder_authority;
        multisig_account.bump = bump;
        multisig_account.validate()?;

        emit!(MultiSigAccountInitializedEvent {
            multisig_account: multisig_account.key(),
        });

        Ok(())
    }

    pub fn update_multi_sig_approval(
        ctx: Context<UpdateMultiSigApproval>,
        grant_approval: bool,
    ) -> Result<()> {
        UpdateMultiSigApproval::auth(&ctx)?;
        let msg;

        let multisig_account = &mut ctx.accounts.multisig_account;

        if grant_approval {
            if multisig_account.proposer.is_none()
                || multisig_account.proposer == Some(ctx.accounts.signer.key())
            {
                msg = "Proposal Pending".to_string();
                multisig_account.proposer = Some(ctx.accounts.signer.key());
            } else {
                msg = "Proposal Accepted | Approval granted".to_string();
                multisig_account.proposer = None;
                multisig_account.is_enabled = true;
            }
        } else {
            msg = "Permission Revoked".to_string();
            multisig_account.is_enabled = false;
        }

        multisig_account.validate()?;

        emit!(UpdateMultiSigApprovalEvent {
            multisig_account: multisig_account.key(),
            signer: ctx.accounts.signer.key(),
            msg
        });

        Ok(())
    }

    pub fn update_multi_sig_commission(
        ctx: Context<UpdateMultiSigCommission>,
        validator_commission_bps: Option<u16>,
    ) -> Result<()> {
        UpdateMultiSigCommission::auth(&ctx)?;

        let multisig_account = &mut ctx.accounts.multisig_account;

        if ctx.accounts.signer.key() == multisig_account.validator_authority.key() {
            if let Some(bps) = validator_commission_bps {
                if bps > 10000 {
                    return Err(ErrorCode::MaxCommissionBpsExceeded.into());
                }
                multisig_account.validator_commission_bps = bps;
            }
        } else {
            multisig_account.block_builder_commission_bps =
                ctx.accounts.config.block_builder_commission_bps;
            multisig_account.block_builder_commission_account =
                ctx.accounts.config.block_builder_commission_account;
        }
        emit!(UpdateMultiSigCommissionEvent {
            multisig_account: multisig_account.key(),
            operator_commission: multisig_account.validator_commission_bps,
            block_builder_commission: multisig_account.block_builder_commission_bps,
        });

        Ok(())
    }

    pub fn close_multi_sig_account(ctx: Context<CloseMultiSigAccount>) -> Result<()> {
        CloseMultiSigAccount::auth(&ctx)?;

        let multisig_account = &mut ctx.accounts.multisig_account;

        let amount = MultiSigAccount::claim_expired(
            multisig_account.to_account_info(),
            ctx.accounts.validator_identity_account.to_account_info(),
        )?;
        emit!(MultiSigAccountClosedEvent {
            multisig_account: multisig_account.key(),
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

    #[msg("Unauthorized signer.")]
    Unauthorized,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [Config::SEED],
        bump,
        payer = initializer,
        space = Config::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, Config>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub initializer: Signer<'info>,
}

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut, rent_exempt = enforce)]
    pub config: Account<'info, Config>,
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
pub struct InitializeMultiSigAccount<'info> {
    pub config: Account<'info, Config>,
    #[account(
        init,
        seeds = [
            MultiSigAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump,
        payer = signer,
        space = MultiSigAccount::SIZE,
        rent_exempt = enforce
    )]
    pub multisig_account: Account<'info, MultiSigAccount>,
    pub validator_vote_account: AccountInfo<'info>,
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateMultiSigApproval<'info> {
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [
            MultiSigAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = multisig_account.bump,
        rent_exempt = enforce,
    )]
    pub multisig_account: Account<'info, MultiSigAccount>,
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateMultiSigApproval<'_> {
    fn auth(ctx: &Context<UpdateMultiSigApproval>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.multisig_account.validator_authority.key()
            || ctx.accounts.signer.key()
                == ctx.accounts.multisig_account.block_builder_authority.key()
        {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

#[derive(Accounts)]
pub struct UpdateMultiSigCommission<'info> {
    pub config: Account<'info, Config>,
    #[account(
        mut,
        seeds = [
            MultiSigAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = multisig_account.bump,
        rent_exempt = enforce,
    )]
    pub multisig_account: Account<'info, MultiSigAccount>,
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl UpdateMultiSigCommission<'_> {
    fn auth(ctx: &Context<UpdateMultiSigCommission>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.multisig_account.validator_authority.key()
            || ctx.accounts.signer.key()
                == ctx.accounts.multisig_account.block_builder_authority.key()
        {
            Ok(())
        } else {
            Err(Unauthorized.into())
        }
    }
}

#[derive(Accounts)]
pub struct CloseMultiSigAccount<'info> {
    pub config: Account<'info, Config>,
    #[account(
        mut,
        close = validator_identity_account,
        seeds = [
            MultiSigAccount::SEED,
            validator_identity_account.key().as_ref(),
        ],
        bump = multisig_account.bump,
        rent_exempt = enforce,
    )]
    pub multisig_account: Account<'info, MultiSigAccount>,
    #[account(mut)]
    pub validator_identity_account: AccountInfo<'info>,
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseMultiSigAccount<'_> {
    fn auth(ctx: &Context<CloseMultiSigAccount>) -> Result<()> {
        if ctx.accounts.signer.key() == ctx.accounts.multisig_account.block_builder_authority.key()
        {
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
pub struct MultiSigAccountInitializedEvent {
    pub multisig_account: Pubkey,
}

#[event]
pub struct UpdateMultiSigApprovalEvent {
    pub multisig_account: Pubkey,
    pub signer: Pubkey,
    pub msg: String,
}

#[event]
pub struct UpdateMultiSigCommissionEvent {
    pub multisig_account: Pubkey,
    pub operator_commission: u16,
    pub block_builder_commission: u16,
}

#[event]
pub struct MultiSigAccountClosedEvent {
    pub multisig_account: Pubkey,
    pub amount_claimed: u64,
}
