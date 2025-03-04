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

declare_id!("ArEru7KcVzvVzgukQnJhZE4xsAq43bjz2ZcL1C7Wq9d");

#[program]
pub mod multi_sig {
    use jito_programs_vote_state::VoteState;

    use super::*;
    use crate::ErrorCode::*;

    /// Initialize a singleton instance of the [Config] account.
    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        block_builder_authority: Pubkey,
        block_builder_commission_account: Pubkey,
        block_builder_commission_bps: u16,
        max_validator_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.block_builder_authority = block_builder_authority;
        cfg.block_builder_commission_account = block_builder_commission_account;
        cfg.block_builder_commission_bps = block_builder_commission_bps;
        cfg.max_validator_commission_bps = max_validator_commission_bps;
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
        config.max_validator_commission_bps = new_config.max_validator_commission_bps;
        config.block_builder_commission_bps = new_config.block_builder_commission_bps;
        config.validate()?;

        emit!(ConfigUpdatedEvent {
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

#[error_code]
pub enum ErrorCode {
    #[msg("Account failed validation.")]
    AccountValidationFailure,

    #[msg("Encountered an arithmetic under/overflow error.")]
    ArithmeticError,

    #[msg("Supplied invalid parameters.")]
    InvalidParameters,

    #[msg("The given proof is invalid.")]
    InvalidProof,

    #[msg("Failed to deserialize the supplied vote account data.")]
    InvalidVoteAccountData,

    #[msg("Commission basis points must be less than or equal to the Config account's max_commission_bps.")]
    MaxCommissionFeeBpsExceeded,

    #[msg("The given MultiSigAccount is not ready to be closed.")]
    PrematureCloseMultiSigAccount,

    #[msg("The given ClaimStatus account is not ready to be closed.")]
    PrematureCloseClaimStatus,

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
        if ctx.accounts.config.authority != ctx.accounts.authority.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}
