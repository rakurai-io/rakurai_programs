use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::state::{
    Config, GlobalConfig, ValidatorConfig, ValidatorProposal, GLOBAL_CONFIG_SEED,
    VALIDATOR_CONFIG_SEED, VALIDATOR_PROPOSAL_SEED,
};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    name: "Rakurai Client Config Program",
    project_url: "https://rakurai.io/",
    contacts: "link:https://rakurai.io/company,link:https://discord.gg/QzqQVBAMpp,link:https://t.me/rakurai_official,link:https://github.com/rakurai-io/rakurai-validator,link:https://docs.rakurai.io",
    policy: "https://rakurai.io/faqs",
    preferred_languages: "en",
    source_code: "https://github.com/rakurai-io/rakurai_programs"
}

pub mod sdk;
pub mod state;

declare_id!("FcTL7Mnq1RcstcYUk39ph2DzdVPNFyWh1EnrqCocXhhh");

#[program]
pub mod rakurai_client_config {
    use super::*;

    /// Create the singleton global config. Signer becomes `manager`.
    pub fn init_global(ctx: Context<InitGlobal>, config: Config) -> Result<()> {
        config.validate()?;
        let global = &mut ctx.accounts.global;
        global.manager = ctx.accounts.manager.key();
        global.bump = ctx.bumps.global;
        global.config = config;
        GlobalConfig::realloc_to_fit(
            &ctx.accounts.global,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: replace global config payload.
    /// Reallocs the account to the serialized size of the new config (dynamic Vec/String).
    pub fn update_global(ctx: Context<UpdateGlobal>, config: Config) -> Result<()> {
        config.validate()?;
        ctx.accounts.global.config = config;
        GlobalConfig::realloc_to_fit(
            &ctx.accounts.global,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: create per-vote validator PDA, copying current global config.
    /// `operator` may later propose changes via the proposal PDA.
    pub fn init_validator(ctx: Context<InitValidator>, operator: Pubkey) -> Result<()> {
        let global = &ctx.accounts.global;
        let validator = &mut ctx.accounts.validator;
        validator.manager = global.manager;
        validator.operator = operator;
        validator.vote = ctx.accounts.vote.key();
        validator.bump = ctx.bumps.validator;
        validator.config = global.config.clone();
        ValidatorConfig::realloc_to_fit(
            &ctx.accounts.validator,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: replace validator-specific config (independent of global after copy).
    /// Reallocs the account to the serialized size of the new config (dynamic Vec/String).
    pub fn update_validator(ctx: Context<UpdateValidator>, config: Config) -> Result<()> {
        config.validate()?;
        ctx.accounts.validator.config = config;
        ValidatorConfig::realloc_to_fit(
            &ctx.accounts.validator,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: set who may propose for this vote.
    pub fn set_operator(ctx: Context<SetOperator>, operator: Pubkey) -> Result<()> {
        ctx.accounts.validator.operator = operator;
        Ok(())
    }

    /// Manager-only: close global config PDA and reclaim rent.
    pub fn close_global(_ctx: Context<CloseGlobal>) -> Result<()> {
        Ok(())
    }

    /// Manager-only: close per-vote validator config PDA and reclaim rent.
    pub fn close_validator(_ctx: Context<CloseValidator>) -> Result<()> {
        Ok(())
    }

    /// Operator-only: create proposal PDA with suggested config.
    pub fn init_proposal(ctx: Context<InitProposal>, config: Config) -> Result<()> {
        config.validate()?;
        let proposal = &mut ctx.accounts.proposal;
        proposal.vote = ctx.accounts.vote.key();
        proposal.operator = ctx.accounts.operator.key();
        proposal.bump = ctx.bumps.proposal;
        proposal.config = config;
        ValidatorProposal::realloc_to_fit(
            &ctx.accounts.proposal,
            &ctx.accounts.operator,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Operator-only: replace proposal payload (reallocs).
    pub fn update_proposal(ctx: Context<UpdateProposal>, config: Config) -> Result<()> {
        config.validate()?;
        ctx.accounts.proposal.config = config;
        ValidatorProposal::realloc_to_fit(
            &ctx.accounts.proposal,
            &ctx.accounts.operator,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: copy proposal → live validator config, then close proposal (rent → operator).
    pub fn approve_proposal(ctx: Context<ApproveProposal>) -> Result<()> {
        let proposed = ctx.accounts.proposal.config.clone();
        proposed.validate()?;
        ctx.accounts.validator.config = proposed;
        ValidatorConfig::realloc_to_fit(
            &ctx.accounts.validator,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: close proposal without changing live validator config (rent → operator).
    pub fn reject_proposal(_ctx: Context<RejectProposal>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(config: Config)]
pub struct InitGlobal<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        init,
        payer = manager,
        space = GlobalConfig::init_space(manager.key(), &config)?,
        seeds = [GLOBAL_CONFIG_SEED],
        bump
    )]
    pub global: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateGlobal<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(operator: Pubkey)]
pub struct InitValidator<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        init,
        payer = manager,
        space = ValidatorConfig::init_space(
            global.manager,
            operator,
            vote.key(),
            &global.config,
        )?,
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump
    )]
    pub validator: Account<'info, ValidatorConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateValidator<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        mut,
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetOperator<'info> {
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        mut,
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
}

#[derive(Accounts)]
pub struct CloseGlobal<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        mut,
        close = manager,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
}

#[derive(Accounts)]
pub struct CloseValidator<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        mut,
        close = manager,
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
}

#[derive(Accounts)]
#[instruction(config: Config)]
pub struct InitProposal<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        has_one = operator @ ConfigError::UnauthorizedOperator,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    #[account(
        init,
        payer = operator,
        space = ValidatorProposal::init_space(vote.key(), operator.key(), &config)?,
        seeds = [VALIDATOR_PROPOSAL_SEED, vote.key().as_ref()],
        bump
    )]
    pub proposal: Account<'info, ValidatorProposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UpdateProposal<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        has_one = operator @ ConfigError::UnauthorizedOperator,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    #[account(
        mut,
        seeds = [VALIDATOR_PROPOSAL_SEED, vote.key().as_ref()],
        bump = proposal.bump,
        constraint = proposal.vote == vote.key() @ ConfigError::VoteMismatch,
        has_one = operator @ ConfigError::UnauthorizedOperator,
    )]
    pub proposal: Account<'info, ValidatorProposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ApproveProposal<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    /// CHECK: receives proposal rent; must match proposal.operator
    #[account(mut)]
    pub operator: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        mut,
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    #[account(
        mut,
        close = operator,
        seeds = [VALIDATOR_PROPOSAL_SEED, vote.key().as_ref()],
        bump = proposal.bump,
        constraint = proposal.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = proposal.operator == operator.key() @ ConfigError::OperatorMismatch,
    )]
    pub proposal: Account<'info, ValidatorProposal>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RejectProposal<'info> {
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    /// CHECK: receives proposal rent; must match proposal.operator
    #[account(mut)]
    pub operator: UncheckedAccount<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    #[account(
        mut,
        close = operator,
        seeds = [VALIDATOR_PROPOSAL_SEED, vote.key().as_ref()],
        bump = proposal.bump,
        constraint = proposal.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = proposal.operator == operator.key() @ ConfigError::OperatorMismatch,
    )]
    pub proposal: Account<'info, ValidatorProposal>,
}

#[error_code]
pub enum ConfigError {
    #[msg("Only the config manager may perform this action")]
    Unauthorized,
    #[msg("Only the validator operator may propose")]
    UnauthorizedOperator,
    #[msg("Vote account does not match validator config")]
    VoteMismatch,
    #[msg("Operator account does not match proposal")]
    OperatorMismatch,
    #[msg("URL must not be empty")]
    UrlEmpty,
}
