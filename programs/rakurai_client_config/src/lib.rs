use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::state::{
    Config, ConfigLimits, ConfigStaging, GlobalConfig, ValidatorConfig, ValidatorProposal,
    CONFIG_STAGING_SEED, GLOBAL_CONFIG_SEED, MAX_STAGING_BYTES, STAGING_KIND_GLOBAL,
    STAGING_KIND_PROPOSAL, STAGING_KIND_VALIDATOR, STAGING_TAG_GLOBAL, STAGING_TAG_PROPOSAL,
    STAGING_TAG_VALIDATOR, VALIDATOR_CONFIG_SEED, VALIDATOR_PROPOSAL_SEED,
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
    /// `limits` must be non-zero and ≤ absolute safety caps.
    pub fn init_global(
        ctx: Context<InitGlobal>,
        config: Config,
        limits: ConfigLimits,
    ) -> Result<()> {
        limits.validate()?;
        config.validate(&limits)?;
        let global = &mut ctx.accounts.global;
        global.manager = ctx.accounts.manager.key();
        global.bump = ctx.bumps.global;
        global.limits = limits;
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
        let limits = ctx.accounts.global.limits;
        config.validate(&limits)?;
        ctx.accounts.global.config = config;
        GlobalConfig::realloc_to_fit(
            &ctx.accounts.global,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: update global size caps (≤ absolute safety caps; must fit current payload).
    pub fn update_global_limits(
        ctx: Context<UpdateGlobalLimits>,
        limits: ConfigLimits,
    ) -> Result<()> {
        limits.validate()?;
        ctx.accounts.global.config.validate(&limits)?;
        ctx.accounts.global.limits = limits;
        Ok(())
    }

    /// Manager-only: create per-vote validator PDA, copying current global config + limits.
    /// `operator` may later propose changes via the proposal PDA.
    pub fn init_validator(ctx: Context<InitValidator>, operator: Pubkey) -> Result<()> {
        let global = &ctx.accounts.global;
        let validator = &mut ctx.accounts.validator;
        validator.manager = global.manager;
        validator.operator = operator;
        validator.vote = ctx.accounts.vote.key();
        validator.bump = ctx.bumps.validator;
        validator.limits = global.limits;
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
        let limits = ctx.accounts.validator.limits;
        config.validate(&limits)?;
        ctx.accounts.validator.config = config;
        ValidatorConfig::realloc_to_fit(
            &ctx.accounts.validator,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: update this vote's size caps (≤ absolute safety caps; must fit current payload).
    pub fn update_validator_limits(
        ctx: Context<UpdateValidatorLimits>,
        limits: ConfigLimits,
    ) -> Result<()> {
        limits.validate()?;
        ctx.accounts.validator.config.validate(&limits)?;
        ctx.accounts.validator.limits = limits;
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
    /// Copies validator `limits` onto the proposal; validates against those caps.
    pub fn init_proposal(ctx: Context<InitProposal>, config: Config) -> Result<()> {
        let limits = ctx.accounts.validator.limits;
        config.validate(&limits)?;
        let proposal = &mut ctx.accounts.proposal;
        proposal.vote = ctx.accounts.vote.key();
        proposal.operator = ctx.accounts.operator.key();
        proposal.bump = ctx.bumps.proposal;
        proposal.limits = limits;
        proposal.config = config;
        ValidatorProposal::realloc_to_fit(
            &ctx.accounts.proposal,
            &ctx.accounts.operator,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Operator-only: replace proposal payload (reallocs).
    /// Refreshes proposal `limits` from the validator PDA; validates against those caps.
    pub fn update_proposal(ctx: Context<UpdateProposal>, config: Config) -> Result<()> {
        let limits = ctx.accounts.validator.limits;
        config.validate(&limits)?;
        ctx.accounts.proposal.limits = limits;
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
        let limits = ctx.accounts.validator.limits;
        let proposed = ctx.accounts.proposal.config.clone();
        proposed.validate(&limits)?;
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

    /// Manager-only: create ephemeral global staging PDA for a chunked config upload.
    pub fn init_global_staging(ctx: Context<InitGlobalStaging>, expected_len: u32) -> Result<()> {
        require!(
            expected_len > 0 && expected_len <= MAX_STAGING_BYTES,
            ConfigError::StagingTooLarge
        );
        let staging = &mut ctx.accounts.staging;
        staging.authority = ctx.accounts.manager.key();
        staging.bump = ctx.bumps.staging;
        staging.kind = STAGING_KIND_GLOBAL;
        staging.vote = Pubkey::default();
        staging.expected_len = expected_len;
        staging.data = Vec::new();
        Ok(())
    }

    /// Manager-only: append a chunk to global staging.
    pub fn write_global_staging(ctx: Context<WriteGlobalStaging>, data: Vec<u8>) -> Result<()> {
        ctx.accounts.staging.append(&data)?;
        ConfigStaging::realloc_to_fit(
            &ctx.accounts.staging,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: deserialize staging payload into live global config, then close staging.
    pub fn commit_global_staging(ctx: Context<CommitGlobalStaging>) -> Result<()> {
        require!(
            ctx.accounts.staging.kind == STAGING_KIND_GLOBAL,
            ConfigError::StagingKindInvalid
        );
        let config = ctx.accounts.staging.parse_config()?;
        let limits = ctx.accounts.global.limits;
        config.validate(&limits)?;
        ctx.accounts.global.config = config;
        GlobalConfig::realloc_to_fit(
            &ctx.accounts.global,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: close global staging without applying.
    pub fn abort_global_staging(_ctx: Context<AbortGlobalStaging>) -> Result<()> {
        Ok(())
    }

    /// Manager-only: create ephemeral validator staging PDA.
    pub fn init_validator_staging(
        ctx: Context<InitValidatorStaging>,
        expected_len: u32,
    ) -> Result<()> {
        require!(
            expected_len > 0 && expected_len <= MAX_STAGING_BYTES,
            ConfigError::StagingTooLarge
        );
        let staging = &mut ctx.accounts.staging;
        staging.authority = ctx.accounts.manager.key();
        staging.bump = ctx.bumps.staging;
        staging.kind = STAGING_KIND_VALIDATOR;
        staging.vote = ctx.accounts.vote.key();
        staging.expected_len = expected_len;
        staging.data = Vec::new();
        Ok(())
    }

    /// Manager-only: append a chunk to validator staging.
    pub fn write_validator_staging(
        ctx: Context<WriteValidatorStaging>,
        data: Vec<u8>,
    ) -> Result<()> {
        ctx.accounts.staging.append(&data)?;
        ConfigStaging::realloc_to_fit(
            &ctx.accounts.staging,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: apply staging payload to live validator overlay, then close staging.
    pub fn commit_validator_staging(ctx: Context<CommitValidatorStaging>) -> Result<()> {
        require!(
            ctx.accounts.staging.kind == STAGING_KIND_VALIDATOR,
            ConfigError::StagingKindInvalid
        );
        require!(
            ctx.accounts.staging.vote == ctx.accounts.vote.key(),
            ConfigError::VoteMismatch
        );
        let config = ctx.accounts.staging.parse_config()?;
        let limits = ctx.accounts.validator.limits;
        config.validate(&limits)?;
        ctx.accounts.validator.config = config;
        ValidatorConfig::realloc_to_fit(
            &ctx.accounts.validator,
            &ctx.accounts.manager,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Manager-only: close validator staging without applying.
    pub fn abort_validator_staging(_ctx: Context<AbortValidatorStaging>) -> Result<()> {
        Ok(())
    }

    /// Operator-only: create ephemeral proposal staging PDA.
    pub fn init_proposal_staging(
        ctx: Context<InitProposalStaging>,
        expected_len: u32,
    ) -> Result<()> {
        require!(
            expected_len > 0 && expected_len <= MAX_STAGING_BYTES,
            ConfigError::StagingTooLarge
        );
        let staging = &mut ctx.accounts.staging;
        staging.authority = ctx.accounts.operator.key();
        staging.bump = ctx.bumps.staging;
        staging.kind = STAGING_KIND_PROPOSAL;
        staging.vote = ctx.accounts.vote.key();
        staging.expected_len = expected_len;
        staging.data = Vec::new();
        Ok(())
    }

    /// Operator-only: append a chunk to proposal staging.
    pub fn write_proposal_staging(ctx: Context<WriteProposalStaging>, data: Vec<u8>) -> Result<()> {
        ctx.accounts.staging.append(&data)?;
        ConfigStaging::realloc_to_fit(
            &ctx.accounts.staging,
            &ctx.accounts.operator,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Operator-only: apply staging payload onto the proposal PDA, then close staging.
    /// Proposal must already exist (small `init_proposal` with empty config is enough).
    pub fn commit_proposal_staging(ctx: Context<CommitProposalStaging>) -> Result<()> {
        require!(
            ctx.accounts.staging.kind == STAGING_KIND_PROPOSAL,
            ConfigError::StagingKindInvalid
        );
        require!(
            ctx.accounts.staging.vote == ctx.accounts.vote.key(),
            ConfigError::VoteMismatch
        );
        let limits = ctx.accounts.validator.limits;
        let config = ctx.accounts.staging.parse_config()?;
        config.validate(&limits)?;
        ctx.accounts.proposal.limits = limits;
        ctx.accounts.proposal.config = config;
        ValidatorProposal::realloc_to_fit(
            &ctx.accounts.proposal,
            &ctx.accounts.operator,
            &ctx.accounts.system_program,
        )?;
        Ok(())
    }

    /// Operator-only: close proposal staging without applying.
    pub fn abort_proposal_staging(_ctx: Context<AbortProposalStaging>) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(config: Config, limits: ConfigLimits)]
pub struct InitGlobal<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        init,
        payer = manager,
        space = GlobalConfig::init_space(manager.key(), limits, &config)?,
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
pub struct UpdateGlobalLimits<'info> {
    pub manager: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
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
            global.limits,
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
pub struct UpdateValidatorLimits<'info> {
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
        space = ValidatorProposal::init_space(vote.key(), operator.key(), validator.limits, &config)?,
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

#[derive(Accounts)]
#[instruction(expected_len: u32)]
pub struct InitGlobalStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        init,
        payer = manager,
        space = ConfigStaging::init_space(
            manager.key(),
            STAGING_KIND_GLOBAL,
            Pubkey::default(),
            expected_len,
        )?,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_GLOBAL],
        bump
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WriteGlobalStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_GLOBAL],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_GLOBAL @ ConfigError::StagingKindInvalid,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CommitGlobalStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        mut,
        seeds = [GLOBAL_CONFIG_SEED],
        bump = global.bump,
        has_one = manager @ ConfigError::Unauthorized,
    )]
    pub global: Account<'info, GlobalConfig>,
    #[account(
        mut,
        close = manager,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_GLOBAL],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_GLOBAL @ ConfigError::StagingKindInvalid,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AbortGlobalStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    #[account(
        mut,
        close = manager,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_GLOBAL],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_GLOBAL @ ConfigError::StagingKindInvalid,
    )]
    pub staging: Account<'info, ConfigStaging>,
}

#[derive(Accounts)]
#[instruction(expected_len: u32)]
pub struct InitValidatorStaging<'info> {
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
        seeds = [VALIDATOR_CONFIG_SEED, vote.key().as_ref()],
        bump = validator.bump,
        constraint = validator.vote == vote.key() @ ConfigError::VoteMismatch,
        constraint = validator.manager == global.manager @ ConfigError::Unauthorized,
    )]
    pub validator: Account<'info, ValidatorConfig>,
    #[account(
        init,
        payer = manager,
        space = ConfigStaging::init_space(
            manager.key(),
            STAGING_KIND_VALIDATOR,
            vote.key(),
            expected_len,
        )?,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_VALIDATOR, vote.key().as_ref()],
        bump
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WriteValidatorStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_VALIDATOR, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_VALIDATOR @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CommitValidatorStaging<'info> {
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
    #[account(
        mut,
        close = manager,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_VALIDATOR, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_VALIDATOR @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AbortValidatorStaging<'info> {
    #[account(mut)]
    pub manager: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        mut,
        close = manager,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_VALIDATOR, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == manager.key() @ ConfigError::Unauthorized,
        constraint = staging.kind == STAGING_KIND_VALIDATOR @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
}

#[derive(Accounts)]
#[instruction(expected_len: u32)]
pub struct InitProposalStaging<'info> {
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
        space = ConfigStaging::init_space(
            operator.key(),
            STAGING_KIND_PROPOSAL,
            vote.key(),
            expected_len,
        )?,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_PROPOSAL, vote.key().as_ref()],
        bump
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct WriteProposalStaging<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        mut,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_PROPOSAL, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == operator.key() @ ConfigError::UnauthorizedOperator,
        constraint = staging.kind == STAGING_KIND_PROPOSAL @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CommitProposalStaging<'info> {
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
    #[account(
        mut,
        close = operator,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_PROPOSAL, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == operator.key() @ ConfigError::UnauthorizedOperator,
        constraint = staging.kind == STAGING_KIND_PROPOSAL @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct AbortProposalStaging<'info> {
    #[account(mut)]
    pub operator: Signer<'info>,
    /// CHECK: vote account used as PDA seed
    pub vote: UncheckedAccount<'info>,
    #[account(
        mut,
        close = operator,
        seeds = [CONFIG_STAGING_SEED, STAGING_TAG_PROPOSAL, vote.key().as_ref()],
        bump = staging.bump,
        constraint = staging.authority == operator.key() @ ConfigError::UnauthorizedOperator,
        constraint = staging.kind == STAGING_KIND_PROPOSAL @ ConfigError::StagingKindInvalid,
        constraint = staging.vote == vote.key() @ ConfigError::VoteMismatch,
    )]
    pub staging: Account<'info, ConfigStaging>,
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
    #[msg("URL exceeds max_url_len")]
    UrlTooLong,
    #[msg("Too many named sets for this section")]
    TooManySets,
    #[msg("Too many URLs in a set")]
    TooManyUrls,
    #[msg("Too many virtual-priority entries in a set")]
    TooManyVpEntries,
    #[msg("virtual_priority value must be in [0.0, 1.0]")]
    InvalidVpValue,
    #[msg("ConfigLimits are zero or exceed absolute safety caps")]
    InvalidLimits,
    #[msg("Staging kind does not match this instruction")]
    StagingKindInvalid,
    #[msg("Staging payload length exceeds expected_len")]
    StagingLengthMismatch,
    #[msg("Staging payload is incomplete")]
    StagingIncomplete,
    #[msg("Staging payload is empty or exceeds MAX_STAGING_BYTES")]
    StagingTooLarge,
    #[msg("Failed to deserialize Config from staging bytes")]
    StagingDeserializeFailed,
}
