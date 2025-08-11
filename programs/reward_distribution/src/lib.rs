use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    state::{ClaimStatus, MerkleRoot, RewardCollectionAccount, RewardDistributionConfigAccount},
    ErrorCode::{InvalidRakuraiCommissionAccount, Unauthorized},
};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai Block Reward Distribution Program",
    project_url: "https://rakurai.io/",
    contacts: "https://rakurai.io/company",
    policy: "https://rakurai.io/faq"
}
pub mod merkle_proof;
pub mod sdk;
pub mod state;

declare_id!("A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB");

#[program]
pub mod reward_distribution {
    use rakurai_vote_state::VoteState;
    use solana_program::{program::invoke, system_instruction};

    use super::*;
    use crate::ErrorCode::*;

    /// Sets up the singleton [RewardDistributionConfigAccount] to store global configuration settings for Rakurai.
    pub fn initialize(
        ctx: Context<Initialize>,
        authority: Pubkey,
        num_epochs_valid: u64,
        max_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.config;
        cfg.authority = authority;
        cfg.num_epochs_valid = num_epochs_valid;
        cfg.max_commission_bps = max_commission_bps;
        cfg.bump = bump;
        cfg.validate()?;

        Ok(())
    }

    /// Initialize a new [RewardCollectionAccount] associated with the given validator vote key
    /// and current epoch.
    pub fn initialize_reward_collection_account(
        ctx: Context<InitializeRewardCollectionAccount>,
        merkle_root_upload_authority: Pubkey,
        validator_commission_bps: u16,
        rakurai_commission_account: Pubkey,
        rakurai_commission_bps: u16,
        bump: u8,
    ) -> Result<()> {
        if validator_commission_bps > ctx.accounts.config.max_commission_bps
            || rakurai_commission_bps > ctx.accounts.config.max_commission_bps
            || (validator_commission_bps + rakurai_commission_bps)
                > ctx.accounts.config.max_commission_bps
        {
            return Err(MaxCommissionFeeBpsExceeded.into());
        }

        if ctx.accounts.validator_vote_account.owner != &solana_program::vote::program::id() {
            return Err(Unauthorized.into());
        }

        let validator_vote_state =
            VoteState::deserialize(&ctx.accounts.validator_vote_account).unwrap();
        if &validator_vote_state.node_pubkey != ctx.accounts.signer.key {
            return Err(Unauthorized.into());
        }

        let current_epoch = Clock::get()?.epoch;

        let reward_collection_acc = &mut ctx.accounts.reward_collection_account;
        reward_collection_acc.validator_vote_account = ctx.accounts.validator_vote_account.key();
        reward_collection_acc.creation_epoch = current_epoch;
        reward_collection_acc.validator_commission_bps = validator_commission_bps;
        reward_collection_acc.rakurai_commission_bps = rakurai_commission_bps;
        reward_collection_acc.rakurai_commission_account = rakurai_commission_account;
        reward_collection_acc.merkle_root_upload_authority = merkle_root_upload_authority;
        reward_collection_acc.merkle_root = None;
        reward_collection_acc.expires_at = current_epoch
            .checked_add(ctx.accounts.config.num_epochs_valid)
            .ok_or(ArithmeticError)?;
        reward_collection_acc.initializer = ctx.accounts.signer.key();
        reward_collection_acc.bump = bump;
        reward_collection_acc.validate()?;

        emit!(RewardCollectionAccountInitializedEvent {
            reward_collection_account: reward_collection_acc.key(),
        });

        Ok(())
    }

    /// Update config fields. Only the [RewardDistributionConfigAccount] authority can invoke this.
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_config: RewardDistributionConfigAccount,
    ) -> Result<()> {
        UpdateConfig::auth(&ctx)?;

        let config = &mut ctx.accounts.config;
        config.authority = new_config.authority;
        config.num_epochs_valid = new_config.num_epochs_valid;
        config.max_commission_bps = new_config.max_commission_bps;
        config.validate()?;

        emit!(ConfigUpdatedEvent {
            authority: ctx.accounts.authority.key(),
        });

        Ok(())
    }

    /// Uploads a merkle root to the [RewardCollectionAccount]. Only the `merkle_root_upload_authority` can invole this instruction.
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

    /// Transfer staker rewards according to the commission to the [RewardCollectionAccount]. This is invoked every leader turn.
    pub fn transfer_staker_rewards(
        ctx: Context<TransferStakerRewards>,
        total_rewards: u64,
    ) -> Result<()> {
        TransferStakerRewards::auth(&ctx)?;

        if total_rewards <= 0 {
            return Err(RewardsTooLow.into());
        }

        let reward_collection_acc = &ctx.accounts.reward_collection_account;
        let block_builder_fee = total_rewards
            .checked_mul(reward_collection_acc.rakurai_commission_bps as u64)
            .ok_or(ArithmeticError)?
            .checked_div(10_000)
            .ok_or(ArithmeticError)?;

        let remaining = total_rewards
            .checked_sub(block_builder_fee)
            .ok_or(ArithmeticError)?;

        let validator_fee = remaining
            .checked_mul(reward_collection_acc.validator_commission_bps as u64)
            .ok_or(ArithmeticError)?
            .checked_div(10_000)
            .ok_or(ArithmeticError)?;
        let staker_rewards = remaining
            .checked_sub(validator_fee)
            .ok_or(ArithmeticError)?;
        if block_builder_fee + validator_fee + staker_rewards != total_rewards {
            return Err(ArithmeticError.into());
        }
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.signer.key(),
                &&ctx
                    .accounts
                    .reward_collection_account
                    .rakurai_commission_account
                    .key(),
                block_builder_fee,
            ),
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.rakurai_commission_account.clone(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.signer.key(),
                &ctx.accounts
                    .reward_collection_account
                    .to_account_info()
                    .key(),
                staker_rewards,
            ),
            &[
                ctx.accounts.signer.to_account_info(),
                ctx.accounts.reward_collection_account.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        emit!(StakerRewardsTransferredEvent { staker_rewards });

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
}

/// Custom errors for Rakurai activation instructions.
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

    #[msg("Rakurai's commission account must be equal to the RewardCollectionAccount account's rakurai_commission_account.")]
    InvalidRakuraiCommissionAccount,
}

/// Closes a `ClaimStatus` account and refunds lamports to the payer.
#[derive(Accounts)]
pub struct CloseClaimStatus<'info> {
    /// The global configuration account for Rakurai settings.
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
    /// The global configuration account for Rakurai settings.
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

/// Initializes a new reward collection account for a validator at the current epoch.
#[derive(Accounts)]
#[instruction(
    _merkle_root_upload_authority: Pubkey,
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRewardCollectionAccount<'info> {
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

    #[account(
        init,
        seeds = [
            RewardCollectionAccount::SEED,
            validator_vote_account.key().as_ref(),
            Clock::get().unwrap().epoch.to_le_bytes().as_ref(),
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

/// Updates fields in the global reward distribution config.
/// Requires the authority stored in the config to sign.
#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    /// The global configuration account for Rakurai settings.
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

/// Instruction to close a reward collection account after the epoch has ended.
#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct CloseRewardCollectionAccount<'info> {
    /// The global configuration account for Rakurai settings.
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
    /// The global configuration account for Rakurai settings.
    pub config: Account<'info, RewardDistributionConfigAccount>,

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
    /// The global configuration account for Rakurai settings.
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

/// Accounts required to transfer staker rewards with Rakurai commission applied.
#[derive(Accounts)]
pub struct TransferStakerRewards<'info> {
    /// CHECK:
    #[account(mut)]
    pub rakurai_commission_account: AccountInfo<'info>,

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
        } else if ctx.accounts.rakurai_commission_account.key()
            != ctx
                .accounts
                .reward_collection_account
                .rakurai_commission_account
        {
            Err(InvalidRakuraiCommissionAccount.into())
        } else {
            Ok(())
        }
    }
}

// Events

// Emitted when a new RewardCollectionAccount is initialized.
#[event]
pub struct RewardCollectionAccountInitializedEvent {
    /// The newly initialized reward colection account.
    pub reward_collection_account: Pubkey,
}

// Emitted when validator commission basis points are updated.
#[event]
pub struct ValidatorCommissionBpsUpdatedEvent {
    pub reward_collection_account: Pubkey,
    pub old_commission_bps: u16,
    pub new_commission_bps: u16,
}

// Emitted when the Merkle root upload authority is changed.
#[event]
pub struct MerkleRootUploadAuthorityUpdatedEvent {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

// Emitted when a config value is updated by an authorized entity.
#[event]
pub struct ConfigUpdatedEvent {
    /// Who updated it.
    authority: Pubkey,
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

// Emitted when a portion of funds is transferred to the staker.
#[event]
pub struct StakerRewardsTransferredEvent {
    pub staker_rewards: u64,
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
