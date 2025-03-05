use anchor_lang::prelude::*;
#[cfg(not(feature = "no-entrypoint"))]
use {default_env::default_env, solana_security_txt::security_txt};

use crate::{
    state::{ClaimStatus, Config, MerkleRoot, RewardDistributionAccount},
    ErrorCode::Unauthorized,
};

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Rakurai Block Reward Distribution Program",
    project_url: "https://rakurai.io/",
    contacts: "https://rakurai.io/company",
    policy: "https://rakurai.io/faq",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/rakurai-io/rakurai_programs",
    source_revision: default_env!("GIT_SHA", "GIT_SHA_MISSING"),
    source_release: default_env!("GIT_REF_NAME", "GIT_REF_NAME_MISSING")
}
pub mod merkle_proof;
pub mod sdk;
pub mod state;

declare_id!("ArEru7KcVzvVzgukQnJhZE4xsAq43bjz2ZcL1C7Wq9d");

#[program]
pub mod reward_distribution {
    use rakurai_vote_state::VoteState;

    use super::*;
    use crate::ErrorCode::*;

    /// Initialize a singleton instance of the [Config] account.
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

    /// Initialize a new [RewardDistributionAccount] associated with the given validator vote key
    /// and current epoch.
    pub fn initialize_reward_distribution_account(
        ctx: Context<InitializeRewardDistributionAccount>,
        merkle_root_upload_authority: Pubkey,
        validator_commission_bps: u16,
        rakurai_commission_pubkey: Pubkey,
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

        let validator_vote_state = VoteState::deserialize(&ctx.accounts.validator_vote_account)?;
        if &validator_vote_state.node_pubkey != ctx.accounts.signer.key {
            return Err(Unauthorized.into());
        }

        let current_epoch = Clock::get()?.epoch;

        let distribution_acc = &mut ctx.accounts.reward_distribution_account;
        distribution_acc.validator_vote_account = ctx.accounts.validator_vote_account.key();
        distribution_acc.epoch_created_at = current_epoch;
        distribution_acc.validator_commission_bps = validator_commission_bps;
        distribution_acc.rakurai_commission_bps = rakurai_commission_bps;
        distribution_acc.rakurai_commission_pubkey = rakurai_commission_pubkey;
        distribution_acc.merkle_root_upload_authority = merkle_root_upload_authority;
        distribution_acc.merkle_root = None;
        distribution_acc.expires_at = current_epoch
            .checked_add(ctx.accounts.config.num_epochs_valid)
            .ok_or(ArithmeticError)?;
        distribution_acc.expired_funds_account = ctx.accounts.signer.key();
        distribution_acc.bump = bump;
        distribution_acc.validate()?;

        emit!(RewardDistributionAccountInitializedEvent {
            reward_distribution_account: distribution_acc.key(),
        });

        Ok(())
    }

    /// Update config fields. Only the [Config] authority can invoke this.
    pub fn update_config(ctx: Context<UpdateConfig>, new_config: Config) -> Result<()> {
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

    /// Uploads a merkle root to the provided [RewardDistributionAccount]. This instruction may be
    /// invoked many times as long as the account is at least one epoch old and not expired; and
    /// no funds have already been claimed. Only the `merkle_root_upload_authority` has the
    /// authority to invoke.
    pub fn upload_merkle_root(
        ctx: Context<UploadMerkleRoot>,
        root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
    ) -> Result<()> {
        UploadMerkleRoot::auth(&ctx)?;

        let current_epoch = Clock::get()?.epoch;
        let distribution_acc = &mut ctx.accounts.reward_distribution_account;

        if let Some(merkle_root) = &distribution_acc.merkle_root {
            if merkle_root.num_nodes_claimed > 0 {
                return Err(Unauthorized.into());
            }
        }
        if current_epoch <= distribution_acc.epoch_created_at {
            return Err(PrematureMerkleRootUpload.into());
        }

        if current_epoch > distribution_acc.expires_at {
            return Err(ExpiredRewardDistributionAccount.into());
        }

        distribution_acc.merkle_root = Some(MerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
            total_funds_claimed: 0,
            num_nodes_claimed: 0,
        });
        distribution_acc.validate()?;

        emit!(MerkleRootUploadedEvent {
            merkle_root_upload_authority: ctx.accounts.merkle_root_upload_authority.key(),
            reward_distribution_account: distribution_acc.key(),
        });

        Ok(())
    }

    /// Anyone can invoke this only after the [RewardDistributionAccount] has expired.
    /// This instruction will return any rent back to `claimant` and close the account
    pub fn close_claim_status(ctx: Context<CloseClaimStatus>) -> Result<()> {
        let claim_status = &ctx.accounts.claim_status;

        // can only claim after claim_status has expired to prevent draining.
        if Clock::get()?.epoch <= claim_status.expires_at {
            return Err(PrematureCloseClaimStatus.into());
        }

        emit!(ClaimStatusClosedEvent {
            claim_status_payer: ctx.accounts.claim_status_payer.key(),
            claim_status_account: claim_status.key(),
        });

        Ok(())
    }

    /// Anyone can invoke this only after the [RewardDistributionAccount] has expired.
    /// This instruction will send any unclaimed funds to the designated `expired_funds_account`
    /// before closing and returning the rent exempt funds to the validator.
    pub fn close_reward_distribution_account(
        ctx: Context<CloseRewardDistributionAccount>,
        _epoch: u64,
    ) -> Result<()> {
        CloseRewardDistributionAccount::auth(&ctx)?;

        let reward_distribution_account = &mut ctx.accounts.reward_distribution_account;

        if Clock::get()?.epoch <= reward_distribution_account.expires_at {
            return Err(PrematureCloseRewardDistributionAccount.into());
        }

        let expired_amount = RewardDistributionAccount::claim_expired(
            reward_distribution_account.to_account_info(),
            ctx.accounts.expired_funds_account.to_account_info(),
        )?;
        reward_distribution_account.validate()?;

        emit!(RewardDistributionAccountClosedEvent {
            expired_funds_account: ctx.accounts.expired_funds_account.key(),
            reward_distribution_account: reward_distribution_account.key(),
            expired_amount,
        });

        Ok(())
    }

    /// Claims tokens from the [RewardDistributionAccount].
    pub fn claim(ctx: Context<Claim>, bump: u8, amount: u64, proof: Vec<[u8; 32]>) -> Result<()> {
        let claim_status = &mut ctx.accounts.claim_status;
        claim_status.bump = bump;

        let claimant_account = &mut ctx.accounts.claimant;
        let reward_distribution_account = &mut ctx.accounts.reward_distribution_account;

        let clock = Clock::get()?;
        if clock.epoch > reward_distribution_account.expires_at {
            return Err(ExpiredRewardDistributionAccount.into());
        }

        // Redundant check since we shouldn't be able to init a claim status account using the same seeds.
        if claim_status.is_claimed {
            return Err(FundsAlreadyClaimed.into());
        }

        let reward_distribution_info = reward_distribution_account.to_account_info();
        let reward_distribution_epoch_expires_at = reward_distribution_account.expires_at;
        let merkle_root = reward_distribution_account
            .merkle_root
            .as_mut()
            .ok_or(RootNotUploaded)?;

        // Verify the merkle proof.
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

        RewardDistributionAccount::claim(
            reward_distribution_info,
            claimant_account.to_account_info(),
            amount,
        )?;

        // Mark it claimed.
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
            reward_distribution_account: reward_distribution_account.key(),
            payer: ctx.accounts.payer.key(),
            claimant: claimant_account.key(),
            amount
        });

        reward_distribution_account.validate()?;

        Ok(())
    }
}

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

    #[msg("The given RewardDistributionAccount has expired.")]
    ExpiredRewardDistributionAccount,

    #[msg(
        "The funds for the given index and RewardDistributionAccount have already been claimed."
    )]
    FundsAlreadyClaimed,

    #[msg("Supplied invalid parameters.")]
    InvalidParameters,

    #[msg("The given proof is invalid.")]
    InvalidProof,

    #[msg("Failed to deserialize the supplied vote account data.")]
    InvalidVoteAccountData,

    #[msg("Validator's commission basis points must be less than or equal to the Config account's max_commission_bps.")]
    MaxCommissionFeeBpsExceeded,

    #[msg("The given RewardDistributionAccount is not ready to be closed.")]
    PrematureCloseRewardDistributionAccount,

    #[msg("The given ClaimStatus account is not ready to be closed.")]
    PrematureCloseClaimStatus,

    #[msg("Must wait till at least one epoch after the reward distribution account was created to upload the merkle root.")]
    PrematureMerkleRootUpload,

    #[msg("No merkle root has been uploaded to the given RewardDistributionAccount.")]
    RootNotUploaded,

    #[msg("Unauthorized signer.")]
    Unauthorized,
}

#[derive(Accounts)]
pub struct CloseClaimStatus<'info> {
    #[account(seeds = [Config::SEED], bump)]
    pub config: Account<'info, Config>,

    // bypass seed check since owner check prevents attacker from passing in invalid data
    // account can only be transferred to us if it is zeroed, failing the deserialization check
    #[account(
        mut,
        close = claim_status_payer,
        constraint = claim_status_payer.key() == claim_status.claim_status_payer
    )]
    pub claim_status: Account<'info, ClaimStatus>,

    /// CHECK: This is checked against claim_status in the constraint
    /// Receiver of the funds.
    #[account(mut)]
    pub claim_status_payer: UncheckedAccount<'info>,
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
#[instruction(
    _merkle_root_upload_authority: Pubkey,
    _validator_commission_bps: u16,
    _bump: u8
)]
pub struct InitializeRewardDistributionAccount<'info> {
    pub config: Account<'info, Config>,

    #[account(
        init,
        seeds = [
            RewardDistributionAccount::SEED,
            validator_vote_account.key().as_ref(),
            Clock::get().unwrap().epoch.to_le_bytes().as_ref(),
        ],
        bump,
        payer = signer,
        space = RewardDistributionAccount::SIZE,
        rent_exempt = enforce
    )]
    pub reward_distribution_account: Account<'info, RewardDistributionAccount>,

    /// CHECK: Safe because we check the vote program is the owner before deserialization.
    /// The validator's vote account is used to check this transaction's signer is also the authorized withdrawer.
    pub validator_vote_account: AccountInfo<'info>,

    /// Must be equal to the supplied validator vote account's authorized withdrawer.
    #[account(mut)]
    pub signer: Signer<'info>,

    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
#[instruction(epoch: u64)]
pub struct CloseRewardDistributionAccount<'info> {
    pub config: Account<'info, Config>,

    /// CHECK: safe see auth fn
    #[account(mut)]
    pub expired_funds_account: AccountInfo<'info>,

    #[account(
        mut,
        close = validator_vote_account,
        seeds = [
            RewardDistributionAccount::SEED,
            validator_vote_account.key().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        bump = reward_distribution_account.bump,
    )]
    pub reward_distribution_account: Account<'info, RewardDistributionAccount>,

    /// CHECK: safe see auth fn
    #[account(mut)]
    pub validator_vote_account: AccountInfo<'info>,

    /// Anyone can crank this instruction.
    #[account(mut)]
    pub signer: Signer<'info>,
}

impl CloseRewardDistributionAccount<'_> {
    fn auth(ctx: &Context<CloseRewardDistributionAccount>) -> Result<()> {
        if ctx
            .accounts
            .reward_distribution_account
            .expired_funds_account
            != ctx.accounts.expired_funds_account.key()
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

#[derive(Accounts)]
#[instruction(_bump: u8, _amount: u64, _proof: Vec<[u8; 32]>)]
pub struct Claim<'info> {
    pub config: Account<'info, Config>,

    #[account(mut, rent_exempt = enforce)]
    pub reward_distribution_account: Account<'info, RewardDistributionAccount>,

    /// Status of the claim. Used to prevent the same party from claiming multiple times.
    #[account(
        init,
        rent_exempt = enforce,
        seeds = [
            ClaimStatus::SEED,
            claimant.key().as_ref(),
            reward_distribution_account.key().as_ref()
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

    /// Who is paying for the claim.
    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UploadMerkleRoot<'info> {
    pub config: Account<'info, Config>,

    #[account(mut, rent_exempt = enforce)]
    pub reward_distribution_account: Account<'info, RewardDistributionAccount>,

    #[account(mut)]
    pub merkle_root_upload_authority: Signer<'info>,
}

impl UploadMerkleRoot<'_> {
    fn auth(ctx: &Context<UploadMerkleRoot>) -> Result<()> {
        if ctx.accounts.merkle_root_upload_authority.key()
            != ctx
                .accounts
                .reward_distribution_account
                .merkle_root_upload_authority
        {
            Err(Unauthorized.into())
        } else {
            Ok(())
        }
    }
}

// Events

#[event]
pub struct RewardDistributionAccountInitializedEvent {
    pub reward_distribution_account: Pubkey,
}

#[event]
pub struct ValidatorCommissionBpsUpdatedEvent {
    pub reward_distribution_account: Pubkey,
    pub old_commission_bps: u16,
    pub new_commission_bps: u16,
}

#[event]
pub struct MerkleRootUploadAuthorityUpdatedEvent {
    pub old_authority: Pubkey,
    pub new_authority: Pubkey,
}

#[event]
pub struct ConfigUpdatedEvent {
    /// Who updated it.
    authority: Pubkey,
}

#[event]
pub struct ClaimedEvent {
    /// [RewardDistributionAccount] claimed from.
    pub reward_distribution_account: Pubkey,

    /// User that paid for the claim, may or may not be the same as claimant.
    pub payer: Pubkey,

    /// Account that received the funds.
    pub claimant: Pubkey,

    /// Amount of funds to distribute.
    pub amount: u64,
}

#[event]
pub struct MerkleRootUploadedEvent {
    /// Who uploaded the root.
    pub merkle_root_upload_authority: Pubkey,

    /// Where the root was uploaded to.
    pub reward_distribution_account: Pubkey,
}

#[event]
pub struct RewardDistributionAccountClosedEvent {
    /// Account where unclaimed funds were transferred to.
    pub expired_funds_account: Pubkey,

    /// [RewardDistributionAccount] closed.
    pub reward_distribution_account: Pubkey,

    /// Unclaimed amount transferred.
    pub expired_amount: u64,
}

#[event]
pub struct ClaimStatusClosedEvent {
    /// Account where funds were transferred to.
    pub claim_status_payer: Pubkey,

    /// [ClaimStatus] account that was closed.
    pub claim_status_account: Pubkey,
}
