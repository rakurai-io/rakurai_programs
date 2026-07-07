#![allow(unexpected_cfgs)]
use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    state::{
        validate_commission, ClaimStatus, MerkleRoot, RevenueKind, RevenueShareAccount,
        RewardCollectionAccount, RewardDistributionConfigAccount,
    },
    ErrorCode::{InvalidClientCommissionAccount, RakuraiSchedulerNotEnabled, Unauthorized},
};
use rakurai_activation::state::RakuraiActivationAccount;

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai Block Reward Distribution Program",
    project_url: "https://rakurai.io/",
    contacts: "link:https://rakurai.io/company,link:https://github.com/rakurai-io/rakurai-validator,link:https://docs.rakurai.io,discord:https://discord.gg/QzqQVBAMpp,telegram:https://t.me/rakurai_official",
    policy: "https://rakurai.io/faqs",
    // Optional fields
    preferred_languages: "en",
    source_code: "https://github.com/rakurai-io/rakurai_programs"
}
pub mod merkle_proof;
pub mod sdk;
pub mod state;

declare_id!("A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB");

#[program]
pub mod reward_distribution {
    use solana_program::{program::invoke, system_instruction};

    use super::*;
    use crate::ErrorCode::*;

    /// Sets up the singleton [RewardDistributionConfigAccount] to store global configuration settings for Rakurai.
    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        num_epochs_valid: u64,
        max_commission_bps: u16,
        client_commission_on_mev_commission_enabled: bool,
        revenue_manager_authority: Pubkey,
        bump: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.num_epochs_valid = num_epochs_valid;
        cfg.max_commission_bps = max_commission_bps;
        cfg.set_mev_commission_enabled(client_commission_on_mev_commission_enabled);
        cfg.revenue_manager_authority = Some(revenue_manager_authority); // slot reserved in SIZE; set later via update_config
        cfg.bump = bump;
        cfg.validate()?;

        Ok(())
    }

    /// Initialize a new [RewardCollectionAccount] (legacy account list).
    /// Prefer `initialize_reward_collection_account_v1` for enabled RAA validation.
    pub fn initialize_reward_collection_account(
        ctx: Context<InitializeRewardCollectionAccount>,
        merkle_root_upload_authority: Pubkey,
        block_reward_commission_bps: u16,
        client_commission_account: Pubkey,
        client_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        initialize_reward_collection_account_inner(
            &ctx.accounts.config,
            ctx.accounts.reward_collection_account.key(),
            &mut ctx.accounts.reward_collection_account,
            &ctx.accounts.validator_vote_account,
            ctx.accounts.signer.key,
            merkle_root_upload_authority,
            block_reward_commission_bps,
            client_commission_account,
            client_commission_bps,
            bump,
        )
    }

    /// Initialize a new [RewardCollectionAccount] with Rakurai activation checks.
    pub fn initialize_reward_collection_account_v1(
        ctx: Context<InitializeRewardCollectionAccountV1>,
        merkle_root_upload_authority: Pubkey,
        block_reward_commission_bps: u16,
        client_commission_account: Pubkey,
        client_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        initialize_reward_collection_account_inner(
            &ctx.accounts.config,
            ctx.accounts.reward_collection_account.key(),
            &mut ctx.accounts.reward_collection_account,
            &ctx.accounts.validator_vote_account,
            ctx.accounts.signer.key,
            merkle_root_upload_authority,
            block_reward_commission_bps,
            client_commission_account,
            client_commission_bps,
            bump,
        )
    }

    /// Update config fields. Only the [RewardDistributionConfigAccount] authority can invoke this.
    /// Grows legacy config accounts to the current [`RewardDistributionConfigAccount::SIZE`]
    /// (e.g. to persist `revenue_manager_authority`) before applying updates.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_config: RewardDistributionConfigAccount,
    ) -> Result<()> {
        UpdateConfig::auth(&ctx)?;

        let config = &mut ctx.accounts.config;
        config.authority = new_config.authority;
        config.num_epochs_valid = new_config.num_epochs_valid;
        config.max_commission_bps = new_config.max_commission_bps;
        config.set_mev_commission_enabled(new_config.is_mev_commission_enabled());
        config.revenue_manager_authority = new_config.revenue_manager_authority;
        config.validate()?;

        emit!(ConfigUpdatedEvent {
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    /// Closes the reward distribution config account and reclaims rent.
    /// Only the config authority can invoke this instruction.
    pub fn close_config(ctx: Context<CloseConfig>) -> Result<()> {
        // Verify caller authority
        CloseConfig::auth(&ctx)?;

        let config_account = &mut ctx.accounts.config;
        let authority = &mut ctx.accounts.signer;

        // Transfer all lamports from config to authority
        let lamports_to_reclaim = config_account.to_account_info().lamports();
        **config_account.to_account_info().try_borrow_mut_lamports()? = 0;
        **authority.try_borrow_mut_lamports()? = authority
            .lamports()
            .checked_add(lamports_to_reclaim)
            .ok_or(ArithmeticError)?;

        // Emit closure event
        emit!(ConfigClosedEvent {
            authority: authority.key(),
            lamports_reclaimed: lamports_to_reclaim,
        });

        Ok(())
    }

    /// Uploads a merkle root to the [RewardCollectionAccount]. Only the `merkle_root_upload_authority` can invoke this instruction.
    pub fn upload_merkle_root(
        ctx: Context<UploadMerkleRoot>,
        root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
    ) -> Result<()> {
        UploadMerkleRoot::auth(&ctx)?;

        let current_epoch = Clock::get()?.epoch;
        let reward_collection_acc = &mut ctx.accounts.reward_collection_account;

        if let Some(merkle_root) = &reward_collection_acc.merkle_root {
            if merkle_root.num_nodes_claimed > 0 {
                return Err(Unauthorized.into());
            }
        }
        if current_epoch <= reward_collection_acc.creation_epoch {
            return Err(PrematureMerkleRootUpload.into());
        }

        if current_epoch > reward_collection_acc.expires_at {
            return Err(ExpiredRewardCollectionAccount.into());
        }

        let account_info = reward_collection_acc.to_account_info();
        let min_rent = Rent::get()?.minimum_balance(account_info.data_len());
        let spendable =
            RewardCollectionAccount::spendable_lamports(account_info.lamports(), min_rent)?;
        if max_total_claim > spendable {
            return Err(ExceedsMaxClaim.into());
        }

        reward_collection_acc.merkle_root = Some(MerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
            total_funds_claimed: 0,
            num_nodes_claimed: 0,
        });
        reward_collection_acc.validate()?;

        emit!(MerkleRootUploadedEvent {
            merkle_root_upload_authority: ctx.accounts.merkle_root_upload_authority.key(),
            reward_collection_account: reward_collection_acc.key(),
        });

        Ok(())
    }

    /// Transfers staker rewards to the [RewardCollectionAccount] and client commission to commission account from `total_rewards`.
    /// Invoked every leader turn.
    pub fn transfer_staker_rewards(
        ctx: Context<TransferStakerRewards>,
        total_rewards: u64,
    ) -> Result<()> {
        TransferStakerRewards::auth(&ctx)?;

        if total_rewards == 0 {
            return Err(RewardsTooLow.into());
        }

        let reward_collection_acc = &ctx.accounts.reward_collection_account;

        // Calculate client commission (basis points)
        let client_commission_amount = total_rewards
            .checked_mul(reward_collection_acc.client_commission_bps as u64)
            .ok_or(ArithmeticError)?
            .checked_div(10_000)
            .ok_or(ArithmeticError)?;

        let staker_rewards = total_rewards
            .checked_sub(client_commission_amount)
            .ok_or(ArithmeticError)?;

        // Transfer client commission if applicable
        if client_commission_amount > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.signer.key(),
                    &ctx.accounts.client_commission_account.key(),
                    client_commission_amount,
                ),
                &[
                    ctx.accounts.signer.to_account_info(),
                    ctx.accounts.client_commission_account.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }

        // Transfer remaining rewards to [RewardCollectionAccount]
        if staker_rewards > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.signer.key(),
                    &reward_collection_acc.key(),
                    staker_rewards,
                ),
                &[
                    ctx.accounts.signer.to_account_info(),
                    reward_collection_acc.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }

        emit!(StakerRewardsTransferredEvent {
            staker_rewards,
            client_commission_amount,
            total_rewards,
        });

        Ok(())
    }

    /// Deducts client commission from MEV rewards earned by the validator.
    pub fn transfer_client_commission_on_mev_commission(
        ctx: Context<TransferClientCommissionOnMevCommission>,
        mev_rewards: u64,
    ) -> Result<()> {
        TransferClientCommissionOnMevCommission::auth(&ctx)?;

        if mev_rewards == 0 {
            return Err(RewardsTooLow.into());
        }

        let reward_collection_acc = &mut ctx.accounts.reward_collection_account;

        // Prevent double deduction
        if let Some(amount) = reward_collection_acc.client_mev_commission_deducted {
            if amount > 0 {
                return Err(MevCommissionAlreadyDeducted.into());
            }
        }

        // Calculate client commission
        let client_mev_commission = mev_rewards
            .checked_mul(reward_collection_acc.client_commission_bps as u64)
            .ok_or(ArithmeticError)?
            .checked_div(10_000)
            .ok_or(ArithmeticError)?;

        // Transfer commission if > 0
        if client_mev_commission > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.signer.key(),
                    &ctx.accounts.client_commission_account.key(),
                    client_mev_commission,
                ),
                &[
                    ctx.accounts.signer.to_account_info(),
                    ctx.accounts.client_commission_account.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;

            reward_collection_acc.client_mev_commission_deducted = Some(client_mev_commission);
        }

        // Emit event
        emit!(MevCommissionTransferredEvent {
            mev_rewards,
            commission_amount: client_mev_commission,
        });

        Ok(())
    }

    /// Permissionless; can only be invoked once the [`RewardCollectionAccount`] has expired.
    pub fn close_claim_status(ctx: Context<CloseClaimStatus>) -> Result<()> {
        let claim_status = &ctx.accounts.claim_status;

        if Clock::get()?.epoch <= claim_status.expires_at {
            return Err(PrematureCloseClaimStatus.into());
        }

        emit!(ClaimStatusClosedEvent {
            claim_status_payer: ctx.accounts.claim_status_payer.key(),
            claim_status_account: claim_status.key(),
        });

        Ok(())
    }

    /// Sends unclaimed funds to the `initializer` and closes the [`RewardCollectionAccount`],
    /// returning rent to the validator.
    pub fn close_reward_collection_account(
        ctx: Context<CloseRewardCollectionAccount>,
        _epoch: u64,
    ) -> Result<()> {
        CloseRewardCollectionAccount::auth(&ctx)?;

        let reward_collection_account = &mut ctx.accounts.reward_collection_account;

        if Clock::get()?.epoch <= reward_collection_account.expires_at {
            return Err(PrematureCloseRewardCollectionAccount.into());
        }

        let expired_amount = RewardCollectionAccount::claim_expired(
            reward_collection_account.to_account_info(),
            ctx.accounts.initializer.to_account_info(),
        )?;
        reward_collection_account.validate()?;

        emit!(RewardCollectionAccountClosedEvent {
            initializer: ctx.accounts.initializer.key(),
            reward_collection_account: reward_collection_account.key(),
            expired_amount,
        });

        Ok(())
    }

    /// Claims rewards for a staker from the [RewardCollectionAccount] according to their merkle proof.
    pub fn claim(ctx: Context<Claim>, bump: u8, amount: u64, proof: Vec<[u8; 32]>) -> Result<()> {
        let claim_status = &mut ctx.accounts.claim_status;
        claim_status.bump = bump;

        let claimant_account = &mut ctx.accounts.claimant;
        let reward_collection_account = &mut ctx.accounts.reward_collection_account;

        let clock = Clock::get()?;
        if clock.epoch > reward_collection_account.expires_at {
            return Err(ExpiredRewardCollectionAccount.into());
        }

        if claim_status.is_claimed {
            return Err(FundsAlreadyClaimed.into());
        }

        let reward_distribution_info = reward_collection_account.to_account_info();
        let reward_distribution_epoch_expires_at = reward_collection_account.expires_at;
        let merkle_root = reward_collection_account
            .merkle_root
            .as_mut()
            .ok_or(RootNotUploaded)?;

        let node = &solana_program::hash::hashv(&[
            &[0u8],
            &solana_program::hash::hashv(&[
                &claimant_account.key().to_bytes(),
                &amount.to_le_bytes(),
            ])
            .to_bytes(),
        ]);

        if !merkle_proof::verify(proof, merkle_root.root, node.to_bytes()) {
            return Err(InvalidProof.into());
        }

        RewardCollectionAccount::claim(
            reward_distribution_info,
            claimant_account.to_account_info(),
            amount,
        )?;

        claim_status.amount = amount;
        claim_status.is_claimed = true;
        claim_status.slot_claimed_at = clock.slot;
        claim_status.claimant = claimant_account.key();
        claim_status.claim_status_payer = ctx.accounts.payer.key();
        claim_status.expires_at = reward_distribution_epoch_expires_at;

        merkle_root.total_funds_claimed = merkle_root
            .total_funds_claimed
            .checked_add(amount)
            .ok_or(ArithmeticError)?;
        if merkle_root.total_funds_claimed > merkle_root.max_total_claim {
            return Err(ExceedsMaxClaim.into());
        }

        merkle_root.num_nodes_claimed = merkle_root
            .num_nodes_claimed
            .checked_add(1)
            .ok_or(ArithmeticError)?;
        if merkle_root.num_nodes_claimed > merkle_root.max_num_nodes {
            return Err(ExceedsMaxNumNodes.into());
        }

        emit!(ClaimedEvent {
            reward_collection_account: reward_collection_account.key(),
            payer: ctx.accounts.payer.key(),
            claimant: claimant_account.key(),
            amount
        });

        reward_collection_account.validate()?;

        Ok(())
    }

    /// Initializes a revenue share vault (tip or mev-share) for a validator.
    pub fn initialize_revenue_share_account(
        ctx: Context<InitializeRevenueShareAccount>,
        share_kind: RevenueKind,
        name: [u8; 32],
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
        bump: u8,
    ) -> Result<()> {
        InitializeRevenueShareAccount::auth(
            &ctx,
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
        )?;

        let manager_authority = ctx.accounts.config.require_revenue_manager_authority()?;
        let revenue_share_account = &mut ctx.accounts.revenue_share_account;
        revenue_share_account.populate_on_init(
            share_kind,
            name,
            ctx.accounts.validator_vote_account.key(),
            ctx.accounts.payer.key(),
            manager_authority,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            bump,
        )?;

        emit!(RevenueShareAccountInitializedEvent {
            revenue_share_account: revenue_share_account.key(),
            share_kind,
            name,
            validator_vote: revenue_share_account.validator_vote,
            initializer: ctx.accounts.payer.key(),
            manager_authority,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
        });

        Ok(())
    }

    /// Records revenue for the current epoch (accounting only).
    pub fn record_revenue(ctx: Context<RecordRevenue>, amount: u64) -> Result<()> {
        RecordRevenue::auth(&ctx)?;

        let epoch = Clock::get()?.epoch;
        let revenue_share_account = &mut ctx.accounts.revenue_share_account;
        revenue_share_account.record_revenue(epoch, amount)?;

        emit!(RevenueRecordedEvent {
            revenue_share_account: revenue_share_account.key(),
            share_kind: revenue_share_account.share_kind,
            epoch,
            amount,
        });

        Ok(())
    }

    /// Claims revenue for a completed epoch.
    pub fn claim_revenue(ctx: Context<ClaimRevenue>, epoch: u64) -> Result<()> {
        ClaimRevenue::auth(&ctx)?;

        let revenue_share_account = &mut ctx.accounts.revenue_share_account;
        let share_kind = revenue_share_account.share_kind;
        let commission_bps = revenue_share_account.commission_bps;
        let revenue_share_account_info = revenue_share_account.to_account_info();
        let (commission_amount, validator_amount) = RevenueShareAccount::claim_revenue(
            &mut revenue_share_account.ledger,
            revenue_share_account_info,
            ctx.accounts.commission_account.to_account_info(),
            ctx.accounts.validator_identity.to_account_info(),
            commission_bps,
            epoch,
        )?;

        emit!(RevenueClaimedEvent {
            revenue_share_account: revenue_share_account.key(),
            share_kind,
            validator_identity: ctx.accounts.validator_identity.key(),
            commission_account: ctx.accounts.commission_account.key(),
            epoch,
            commission_amount,
            validator_amount,
        });

        Ok(())
    }

    /// Updates revenue share config (`commission_bps`, `commission_account`, `block_reward_conversion_enabled`). Manager authority only.
    pub fn update_revenue_share_config(
        ctx: Context<UpdateRevenueShareConfig>,
        commission_bps: u16,
        commission_account: Pubkey,
        block_reward_conversion_enabled: bool,
        record_authority: Option<Pubkey>,
    ) -> Result<()> {
        UpdateRevenueShareConfig::auth(&ctx, commission_bps, commission_account)?;

        let revenue_share_account = &mut ctx.accounts.revenue_share_account;
        revenue_share_account.update_commission(
            commission_bps,
            commission_account,
            block_reward_conversion_enabled,
            ctx.accounts.manager_authority.key(),
            record_authority,
        )?;

        emit!(RevenueShareConfigUpdatedEvent {
            revenue_share_account: revenue_share_account.key(),
            share_kind: revenue_share_account.share_kind,
            commission_bps,
            commission_account,
            block_reward_conversion_enabled,
            record_authority: revenue_share_account.record_authority,
        });

        Ok(())
    }

    /// Marks a claimed epoch ledger entry as `block_reward_converted`.
    /// Requires entry claimed and entry flag still false.
    /// Callable by manager, record authority, or validator identity (vote node signer).
    pub fn update_epoch_converted_to_block_reward(
        ctx: Context<UpdateEpochConvertedToBlockReward>,
        epoch: u64,
    ) -> Result<()> {
        UpdateEpochConvertedToBlockReward::auth(&ctx)?;

        let revenue_share_account = &mut ctx.accounts.revenue_share_account;
        revenue_share_account.mark_epoch_converted_to_block_reward(epoch)?;

        emit!(RevenueEpochConvertedToBlockRewardUpdatedEvent {
            revenue_share_account: revenue_share_account.key(),
            share_kind: revenue_share_account.share_kind,
            epoch,
            authority: ctx.accounts.signer.key(),
        });

        Ok(())
    }

    /// Closes a revenue share account; rent is returned to the original initializer.
    pub fn close_revenue_share_account(ctx: Context<CloseRevenueShareAccount>) -> Result<()> {
        CloseRevenueShareAccount::auth(&ctx)?;
        Ok(())
    }
}

fn initialize_reward_collection_account_inner(
    config: &RewardDistributionConfigAccount,
    reward_collection_account_pubkey: Pubkey,
    reward_collection_account: &mut RewardCollectionAccount,
    validator_vote_account: &AccountInfo,
    signer: &Pubkey,
    merkle_root_upload_authority: Pubkey,
    block_reward_commission_bps: u16,
    client_commission_account: Pubkey,
    client_commission_bps: u16,
    bump: u8,
) -> Result<()> {
    use rakurai_vote_state::VoteState;

    if block_reward_commission_bps > config.max_commission_bps
        || client_commission_bps > config.max_commission_bps
    {
        return Err(ErrorCode::MaxCommissionFeeBpsExceeded.into());
    }

    if validator_vote_account.owner != &solana_program::vote::program::id() {
        return Err(ErrorCode::Unauthorized.into());
    }

    let node_pubkey = VoteState::deserialize_node_pubkey(validator_vote_account).unwrap();
    if &node_pubkey != signer {
        return Err(ErrorCode::Unauthorized.into());
    }

    let current_epoch = Clock::get()?.epoch;

    reward_collection_account.validator_vote_account = validator_vote_account.key();
    reward_collection_account.creation_epoch = current_epoch;
    reward_collection_account.block_reward_commission_bps = block_reward_commission_bps;
    reward_collection_account.client_commission_bps = client_commission_bps;
    reward_collection_account.client_commission_account = client_commission_account;
    reward_collection_account.merkle_root_upload_authority = merkle_root_upload_authority;
    reward_collection_account.merkle_root = None;
    reward_collection_account.expires_at = current_epoch
        .checked_add(config.num_epochs_valid)
        .ok_or(ErrorCode::ArithmeticError)?;
    reward_collection_account.initializer = *signer;
    reward_collection_account.bump = bump;

    if config.is_mev_commission_enabled() {
        reward_collection_account.client_mev_commission_deducted = Some(0);
    } else {
        reward_collection_account.client_mev_commission_deducted = None;
    }

    reward_collection_account.validate()?;

    emit!(RewardCollectionAccountInitializedEvent {
        reward_collection_account: reward_collection_account_pubkey,
    });

    Ok(())
}

/// Custom errors for Reward Distribution Program instructions.
#[error_code]
pub enum ErrorCode {
    #[msg("Account failed validation.")]
    AccountValidationFailure,

    #[msg("Encountered an arithmetic under/overflow error.")]
    ArithmeticError,

    #[msg("The maximum number of funds to be claimed has been exceeded.")]
    ExceedsMaxClaim,

    #[msg("The maximum number of claims has been exceeded.")]
    ExceedsMaxNumNodes,

    #[msg("The given RewardCollectionAccount has expired.")]
    ExpiredRewardCollectionAccount,

    #[msg("The funds for the given index and RewardCollectionAccount have already been claimed.")]
    FundsAlreadyClaimed,

    #[msg("The given proof is invalid.")]
    InvalidProof,

    #[msg("Validator's commission basis points must be less than or equal to the RewardDistributionConfigAccount account's max_commission_bps.")]
    MaxCommissionFeeBpsExceeded,

    #[msg("The given RewardCollectionAccount is not ready to be closed.")]
    PrematureCloseRewardCollectionAccount,

    #[msg("The given ClaimStatus account is not ready to be closed.")]
    PrematureCloseClaimStatus,

    #[msg("Must wait till at least one epoch after the reward distribution account was created to upload the merkle root.")]
    PrematureMerkleRootUpload,

    #[msg("No merkle root has been uploaded to the given RewardCollectionAccount.")]
    RootNotUploaded,

    #[msg("Unauthorized signer.")]
    Unauthorized,

    #[msg("Total rewards must be greater than 0.")]
    RewardsTooLow,

    #[msg("Client commission account must be equal to the RewardCollectionAccount account's client_commission_account.")]
    InvalidClientCommissionAccount,

    #[msg("MEV commission has already been deducted for this epoch")]
    MevCommissionAlreadyDeducted,

    #[msg("Revenue label must be non-empty.")]
    InvalidRevenueName,

    #[msg("Revenue ledger capacity must be between 1 and the program cap.")]
    InvalidRevenueEpochCapacity,

    #[msg("Epoch entry not found in revenue ledger.")]
    EpochEntryNotFound,

    #[msg("Revenue for this epoch has already been claimed.")]
    EpochAlreadyClaimed,

    #[msg("Revenue can only be claimed after the epoch has ended.")]
    PrematureRevenueClaim,

    #[msg("Revenue for this epoch has not been claimed yet.")]
    EpochNotClaimed,

    #[msg("Revenue for this epoch is already marked converted to block rewards.")]
    EpochAlreadyConvertedToBlockReward,

    #[msg("Tip/mev-share revenue manager is not configured on the reward distribution config.")]
    RevenueManagerNotConfigured,

    #[msg("Rakurai scheduler is not enabled for this validator.")]
    RakuraiSchedulerNotEnabled,

    #[msg("Revenue ledger is full and all entries are unclaimed.")]
    RevenueLedgerFull,
}

/// Closes a `ClaimStatus` account and refunds lamports to the payer.
#[derive(Accounts)]
pub struct CloseClaimStatus<'info> {
    /// The global configuration account for Reward Distribution settings.
    #[account(seeds = [RewardDistributionConfigAccount::SEED], bump)]
    pub config: Account<'info, RewardDistributionConfigAccount>,

    /// The [`ClaimStatus`] account associated with the staker's pubkey is closed in this instruction, returning rent to the original payer (`claim_status_payer`).  
    #[account(
        mut,
        close = claim_status_payer,
        constraint = claim_status_payer.key() == claim_status.claim_status_payer
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    /// CHECK: This is checked against claim_status in the constraint
    /// Account that receives the closed account's lamports.
    #[account(mut)]
    pub claim_status_payer: UncheckedAccount<'info>,
}

/// Initializes the reward distribution config with bump and payer.
#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The global configuration account for Reward Distribution settings.
    #[account(
        init,
        seeds = [RewardDistributionConfigAccount::SEED],
        bump,
        payer = initializer,
        space = RewardDistributionConfigAccount::SIZE,
        rent_exempt = enforce
    )]
    pub config: Account<'info, RewardDistributionConfigAccount>,

    pub system_program: Program<'info, System>,

    /// Fee payer for the initialize transaction
    #[account(mut)]
    pub initializer: Signer<'info>,
}

/// Initializes a new reward collection account for a validator at the current epoch (legacy).
#[derive(Accounts)]
#[instruction(
    _merkle_root_upload_authority: Pubkey,
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRewardCollectionAccount<'info> {
    /// The global configuration account for Reward Distribution settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

    #[account(
        init,
        seeds = [
            RewardCollectionAccount::SEED,
            validator_vote_account.key().as_ref(),
            Clock::get().map(|c| c.epoch).unwrap_or_default().to_le_bytes().as_ref(),
        ],
        bump,
        payer = signer,
        space = RewardCollectionAccount::SIZE,
        rent_exempt = enforce
    )]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    /// CHECK: The validator's vote account (used for metadata and on-chain validation).
    pub validator_vote_account: AccountInfo<'info>,

    /// CHECK: The validator's identity account (used to derive the PDA and verify authority).
    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Initializes a new reward collection account with Rakurai activation checks.
#[derive(Accounts)]
#[instruction(
    _merkle_root_upload_authority: Pubkey,
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRewardCollectionAccountV1<'info> {
    /// The global configuration account for Reward Distribution settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

    #[account(
        init,
        seeds = [
            RewardCollectionAccount::SEED,
            validator_vote_account.key().as_ref(),
            Clock::get().map(|c| c.epoch).unwrap_or_default().to_le_bytes().as_ref(),
        ],
        bump,
        payer = signer,
        space = RewardCollectionAccount::SIZE,
        rent_exempt = enforce
    )]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    #[account(
        seeds = [RakuraiActivationAccount::SEED, signer.key().as_ref()],
        bump = rakurai_activation_account.bump,
        seeds::program = rakurai_activation::ID,
        constraint = rakurai_activation_account.validator_authority == signer.key(),
        constraint = rakurai_activation_account.is_enabled @ RakuraiSchedulerNotEnabled,
    )]
    pub rakurai_activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: The validator's vote account (used for metadata and on-chain validation).
    pub validator_vote_account: AccountInfo<'info>,

    /// CHECK: The validator's identity account (used to derive the PDA and verify authority).
    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Updates fields in the global reward distribution config.
/// Requires the authority stored in the config to sign.
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// The global configuration account for Reward Distribution settings.
    #[account(mut, rent_exempt = enforce)]
    pub config: Account<'info, RewardDistributionConfigAccount>,

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

/// Closes the global reward distribution config account.
/// Only the config authority can invoke this instruction.
#[derive(Accounts)]
pub struct CloseConfig<'info> {
    /// The global configuration account for Reward Distribution settings to be closed.
    #[account(
        mut,
        close = signer,
        seeds = [RewardDistributionConfigAccount::SEED],
        bump = config.bump,
        rent_exempt = enforce
    )]
    pub config: Account<'info, RewardDistributionConfigAccount>,

    /// The authority that can close the config account.
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseConfig<'_> {
    fn auth(ctx: &Context<CloseConfig>) -> Result<()> {
        if ctx.accounts.config.authority != ctx.accounts.signer.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

/// Instruction to close a reward collection account after the epoch has ended.
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct CloseRewardCollectionAccount<'info> {
    /// The global configuration account for Reward Distribution settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

    /// CHECK:
    #[account(mut)]
    pub initializer: AccountInfo<'info>,

    #[account(
        mut,
        close = validator_vote_account,
        seeds = [
            RewardCollectionAccount::SEED,
            validator_vote_account.key().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        bump = reward_collection_account.bump,
    )]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    /// CHECK: safe see auth fn
    #[account(mut)]
    pub validator_vote_account: AccountInfo<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseRewardCollectionAccount<'_> {
    fn auth(ctx: &Context<CloseRewardCollectionAccount>) -> Result<()> {
        if ctx.accounts.reward_collection_account.initializer != ctx.accounts.initializer.key() {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

/// Instruction to claim a portion of the reward collection.
/// A new `ClaimStatus` account is created to prevent double claims.
#[derive(Accounts)]
#[instruction(_bump: u8, _amount: u64, _proof: Vec<[u8; 32]>)]
pub struct Claim<'info> {
    #[account(mut, rent_exempt = enforce)]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    /// Status of the claim. Used to prevent the same party from claiming multiple times.
    #[account(
        init,
        rent_exempt = enforce,
        seeds = [
            ClaimStatus::SEED,
            claimant.key().as_ref(),
            reward_collection_account.key().as_ref()
        ],
        bump,
        space = ClaimStatus::SIZE,
        payer = payer
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    /// CHECK: This is safe.
    /// Receiver of the funds.
    #[account(mut)]
    pub claimant: AccountInfo<'info>,

    /// Fee payer for the claim transaction.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

/// Accounts required to upload a Merkle root for reward distribution.
#[derive(Accounts)]
pub struct UploadMerkleRoot<'info> {
    /// The global configuration account for Reward Distribution settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

    #[account(mut, rent_exempt = enforce)]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    #[account(mut)]
    pub merkle_root_upload_authority: Signer<'info>,
}

impl UploadMerkleRoot<'_> {
    fn auth(ctx: &Context<UploadMerkleRoot>) -> Result<()> {
        if ctx.accounts.merkle_root_upload_authority.key()
            != ctx
                .accounts
                .reward_collection_account
                .merkle_root_upload_authority
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

/// Accounts required to transfer staker rewards with client commission applied.
#[derive(Accounts)]
pub struct TransferStakerRewards<'info> {
    /// CHECK:
    #[account(mut)]
    pub client_commission_account: AccountInfo<'info>,

    #[account(mut, rent_exempt = enforce)]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl TransferStakerRewards<'_> {
    fn auth(ctx: &Context<TransferStakerRewards>) -> Result<()> {
        if ctx.accounts.signer.key() != ctx.accounts.reward_collection_account.initializer {
            Err(Unauthorized.into())
        } else if ctx.accounts.client_commission_account.key()
            != ctx
                .accounts
                .reward_collection_account
                .client_commission_account
        {
            Err(InvalidClientCommissionAccount.into())
        } else {
            Ok(())
        }
    }
}

/// Accounts required to transfer mev commission with client commission applied.
#[derive(Accounts)]
pub struct TransferClientCommissionOnMevCommission<'info> {
    /// CHECK:
    #[account(mut)]
    pub client_commission_account: AccountInfo<'info>,

    #[account(mut, rent_exempt = enforce)]
    pub reward_collection_account: Account<'info, RewardCollectionAccount>,

    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub signer: Signer<'info>,
}

impl TransferClientCommissionOnMevCommission<'_> {
    fn auth(ctx: &Context<TransferClientCommissionOnMevCommission>) -> Result<()> {
        if ctx.accounts.signer.key() != ctx.accounts.reward_collection_account.initializer {
            Err(Unauthorized.into())
        } else if ctx.accounts.client_commission_account.key()
            != ctx
                .accounts
                .reward_collection_account
                .client_commission_account
        {
            Err(InvalidClientCommissionAccount.into())
        } else {
            Ok(())
        }
    }
}

/// Initializes a revenue share vault PDA (tip or mev-share).
#[derive(Accounts)]
#[instruction(share_kind: RevenueKind, name: [u8; 32], _record_authority: Pubkey, max_epoch_entries: u8, _commission_bps: u16, _commission_account: Pubkey, _bump: u8)]
pub struct InitializeRevenueShareAccount<'info> {
    #[account(
        init,
        payer = payer,
        space = RevenueShareAccount::space_for(max_epoch_entries as usize),
        seeds = [
            RevenueShareAccount::SEED,
            share_kind.seed(),
            name.as_ref(),
            validator_vote_account.key().as_ref(),
        ],
        bump,
    )]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    #[account(
        seeds = [RewardDistributionConfigAccount::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, RewardDistributionConfigAccount>,

    #[account(
        seeds = [RakuraiActivationAccount::SEED, rakurai_activation_account.validator_authority.as_ref()],
        bump = rakurai_activation_account.bump,
        seeds::program = rakurai_activation::ID,
        constraint = rakurai_activation_account.is_enabled @ RakuraiSchedulerNotEnabled,
    )]
    pub rakurai_activation_account: Account<'info, RakuraiActivationAccount>,

    /// CHECK: validator vote account used in PDA seeds; node must match RAA validator authority.
    pub validator_vote_account: AccountInfo<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

impl InitializeRevenueShareAccount<'_> {
    fn auth(
        ctx: &Context<InitializeRevenueShareAccount>,
        name: [u8; 32],
        record_authority: Pubkey,
        max_epoch_entries: u8,
        commission_bps: u16,
        commission_account: Pubkey,
    ) -> Result<()> {
        use rakurai_vote_state::VoteState;

        if ctx.accounts.validator_vote_account.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }

        let node_pubkey = VoteState::deserialize_node_pubkey(&ctx.accounts.validator_vote_account)
            .map_err(|_| Unauthorized)?;
        if node_pubkey != ctx.accounts.rakurai_activation_account.validator_authority {
            return Err(Unauthorized.into());
        }

        RevenueShareAccount::validate_init_params(
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            ctx.accounts.config.max_commission_bps,
        )
    }
}

/// Records revenue for the current epoch.
#[derive(Accounts)]
pub struct RecordRevenue<'info> {
    #[account(mut)]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    pub record_authority: Signer<'info>,
}

impl RecordRevenue<'_> {
    fn auth(ctx: &Context<RecordRevenue>) -> Result<()> {
        ctx.accounts
            .revenue_share_account
            .auth_record_signer(ctx.accounts.record_authority.key())
    }
}

/// Claims revenue for a completed epoch.
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct ClaimRevenue<'info> {
    #[account(mut)]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    /// CHECK: must match revenue account commission destination
    #[account(
        mut,
        constraint = commission_account.key() == revenue_share_account.commission_account,
    )]
    pub commission_account: AccountInfo<'info>,

    /// CHECK: validator identity receives the non-commission share of claimed revenue.
    #[account(mut)]
    pub validator_identity: AccountInfo<'info>,

    pub manager_authority: Signer<'info>,
}

impl ClaimRevenue<'_> {
    fn auth(ctx: &Context<ClaimRevenue>) -> Result<()> {
        ctx.accounts
            .revenue_share_account
            .auth_manager_signer(ctx.accounts.manager_authority.key())
    }
}

/// Marks a claimed epoch as converted to block rewards (`block_reward_converted` false → true).
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct UpdateEpochConvertedToBlockReward<'info> {
    #[account(mut)]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    /// CHECK: must match `revenue_share_account.validator_vote` when signer is validator identity.
    pub validator_vote_account: AccountInfo<'info>,

    pub signer: Signer<'info>,
}

impl UpdateEpochConvertedToBlockReward<'_> {
    fn auth(ctx: &Context<UpdateEpochConvertedToBlockReward>) -> Result<()> {
        use rakurai_vote_state::VoteState;

        let revenue_share_account = &ctx.accounts.revenue_share_account;
        let signer = ctx.accounts.signer.key();

        if signer == revenue_share_account.manager_authority
            || signer == revenue_share_account.record_authority
        {
            return Ok(());
        }

        let vote = &ctx.accounts.validator_vote_account;
        if vote.key() != revenue_share_account.validator_vote {
            return Err(Unauthorized.into());
        }
        if vote.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }
        let node = VoteState::deserialize_node_pubkey(vote).map_err(|_| Unauthorized)?;
        if node != signer {
            return Err(Unauthorized.into());
        }

        Ok(())
    }
}

/// Updates revenue share config (`commission_bps`, `commission_account`, `block_reward_conversion_enabled`).
#[derive(Accounts)]
pub struct UpdateRevenueShareConfig<'info> {
    #[account(mut)]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    #[account(
        seeds = [RewardDistributionConfigAccount::SEED],
        bump = config.bump,
    )]
    pub config: Account<'info, RewardDistributionConfigAccount>,

    pub manager_authority: Signer<'info>,
}

impl UpdateRevenueShareConfig<'_> {
    fn auth(
        ctx: &Context<UpdateRevenueShareConfig>,
        commission_bps: u16,
        commission_account: Pubkey,
    ) -> Result<()> {
        ctx.accounts
            .revenue_share_account
            .auth_manager_signer(ctx.accounts.manager_authority.key())?;
        validate_commission(
            commission_bps,
            commission_account,
            ctx.accounts.config.max_commission_bps,
        )
    }
}

/// Closes a revenue share account.
#[derive(Accounts)]
pub struct CloseRevenueShareAccount<'info> {
    #[account(
        mut,
        close = initializer,
        seeds = [
            RevenueShareAccount::SEED,
            revenue_share_account.share_kind.seed(),
            revenue_share_account.name.as_ref(),
            revenue_share_account.validator_vote.as_ref(),
        ],
        bump = revenue_share_account.bump,
    )]
    pub revenue_share_account: Account<'info, RevenueShareAccount>,

    /// CHECK: receives rent from the closed account; must match stored `initializer`.
    #[account(
        mut,
        constraint = initializer.key() == revenue_share_account.initializer @ Unauthorized,
    )]
    pub initializer: AccountInfo<'info>,

    pub authority: Signer<'info>,
}

impl CloseRevenueShareAccount<'_> {
    fn auth(ctx: &Context<CloseRevenueShareAccount>) -> Result<()> {
        ctx.accounts
            .revenue_share_account
            .auth_manager_signer(ctx.accounts.authority.key())
    }
}

// Events

// Emitted when a new RewardCollectionAccount is initialized.
#[event]
pub struct RewardCollectionAccountInitializedEvent {
    /// The newly initialized reward colection account.
    pub reward_collection_account: Pubkey,
}

// Emitted when a config value is updated by an authorized entity.
#[event]
pub struct ConfigUpdatedEvent {
    /// Who updated it.
    authority: Pubkey,
}

#[event]
pub struct ConfigClosedEvent {
    /// Authority that closed the config account.
    pub authority: Pubkey,
    /// Amount of lamports reclaimed.
    pub lamports_reclaimed: u64,
}

// Emitted when a user successfully claims rewards from a reward account.
#[event]
pub struct ClaimedEvent {
    /// [RewardCollectionAccount] claimed from.
    pub reward_collection_account: Pubkey,

    /// User that paid for the claim, may or may not be the same as claimant.
    pub payer: Pubkey,

    /// Account that received the funds.
    pub claimant: Pubkey,

    /// Amount of funds to distribute.
    pub amount: u64,
}

// Emitted when a Merkle root is uploaded to a reward account.
#[event]
pub struct MerkleRootUploadedEvent {
    /// Who uploaded the root.
    pub merkle_root_upload_authority: Pubkey,

    /// Where the root was uploaded to.
    pub reward_collection_account: Pubkey,
}

/// Emitted when staker rewards are transferred after deducting commission.
#[event]
pub struct StakerRewardsTransferredEvent {
    // Total rewards for the last leader turn
    pub total_rewards: u64,
    // Commission amount sent to client
    pub client_commission_amount: u64,
    // Remaining rewards sent to [RewardCollectionAccount]
    pub staker_rewards: u64,
}

/// Emitted when client commission on MEV rewards is transferred.
#[event]
pub struct MevCommissionTransferredEvent {
    // Total MEV rewards earned
    pub mev_rewards: u64,
    // Amount deducted as client commission for total mev rewards.
    pub commission_amount: u64,
}

// Emitted when a reward collection account is closed and unclaimed funds are returned.
#[event]
pub struct RewardCollectionAccountClosedEvent {
    /// Account where unclaimed funds were transferred to.
    pub initializer: Pubkey,

    /// [RewardCollectionAccount] closed.
    pub reward_collection_account: Pubkey,

    /// Unclaimed amount transferred.
    pub expired_amount: u64,
}

// Emitted when a user's ClaimStatus account is closed and remaining funds are returned.
#[event]
pub struct ClaimStatusClosedEvent {
    /// Account where funds were transferred to.
    pub claim_status_payer: Pubkey,

    /// [ClaimStatus] account that was closed.
    pub claim_status_account: Pubkey,
}

#[event]
pub struct RevenueShareAccountInitializedEvent {
    pub revenue_share_account: Pubkey,
    pub share_kind: RevenueKind,
    pub name: [u8; 32],
    pub validator_vote: Pubkey,
    pub initializer: Pubkey,
    pub manager_authority: Pubkey,
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
}

#[event]
pub struct RevenueRecordedEvent {
    pub revenue_share_account: Pubkey,
    pub share_kind: RevenueKind,
    pub epoch: u64,
    pub amount: u64,
}

#[event]
pub struct RevenueClaimedEvent {
    pub revenue_share_account: Pubkey,
    pub share_kind: RevenueKind,
    pub validator_identity: Pubkey,
    pub commission_account: Pubkey,
    pub epoch: u64,
    pub commission_amount: u64,
    pub validator_amount: u64,
}

#[event]
pub struct RevenueShareConfigUpdatedEvent {
    pub revenue_share_account: Pubkey,
    pub share_kind: RevenueKind,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub block_reward_conversion_enabled: bool,
    pub record_authority: Pubkey,
}

#[event]
pub struct RevenueEpochConvertedToBlockRewardUpdatedEvent {
    pub revenue_share_account: Pubkey,
    pub share_kind: RevenueKind,
    pub epoch: u64,
    pub authority: Pubkey,
}
