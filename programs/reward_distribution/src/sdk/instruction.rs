//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::{RevenueKind, RewardDistributionConfigAccount};

/// Arguments for initializing the reward distribution config account.
pub struct InitializeArgs {
    pub authority: Pubkey,
    pub num_epochs_valid: u64,
    pub max_commission_bps: u16,
    pub client_commission_on_mev_commission_enabled: bool,
    pub revenue_manager_authority: Pubkey,
    pub bump: u8,
}

/// Accounts needed to initialize the reward distribution config.
pub struct InitializeAccounts {
    pub config: Pubkey,
    pub system_program: Pubkey,
    pub initializer: Pubkey,
}

/// Builds the instruction to initialize the reward distribution config.
pub fn initialize_ix(
    program_id: Pubkey,
    args: InitializeArgs,
    accounts: InitializeAccounts,
) -> Instruction {
    let InitializeArgs {
        authority,
        num_epochs_valid,
        max_commission_bps,
        client_commission_on_mev_commission_enabled,
        revenue_manager_authority,
        bump,
    } = args;

    let InitializeAccounts {
        config,
        system_program,
        initializer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::Initialize {
            authority,
            num_epochs_valid,
            max_commission_bps,
            client_commission_on_mev_commission_enabled,
            revenue_manager_authority,
            bump,
        }
        .data(),
        accounts: crate::accounts::Initialize {
            config,
            system_program,
            initializer,
        }
        .to_account_metas(None),
    }
}

/// Arguments for initializing the reward collection account.
pub struct InitializeRewardCollectionAccountArgs {
    pub merkle_root_upload_authority: Pubkey,
    pub block_reward_commission_bps: u16,
    pub client_commission_account: Pubkey,
    pub client_commission_bps: u16,
    pub bump: u8,
}

/// Accounts needed to initialize the reward collection account (legacy).
pub struct InitializeRewardCollectionAccountAccounts {
    pub config: Pubkey,
    pub reward_collection_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
}

/// Builds the instruction to initialize the reward collection account (legacy).
pub fn initialize_reward_collection_account_ix(
    program_id: Pubkey,
    args: InitializeRewardCollectionAccountArgs,
    accounts: InitializeRewardCollectionAccountAccounts,
) -> Instruction {
    let InitializeRewardCollectionAccountArgs {
        merkle_root_upload_authority,
        block_reward_commission_bps,
        client_commission_account,
        client_commission_bps,
        bump,
    } = args;

    let InitializeRewardCollectionAccountAccounts {
        config,
        reward_collection_account,
        validator_vote_account,
        signer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRewardCollectionAccount {
            merkle_root_upload_authority,
            block_reward_commission_bps,
            client_commission_account,
            client_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRewardCollectionAccount {
            config,
            reward_collection_account,
            validator_vote_account,
            signer,
            system_program,
        }
        .to_account_metas(None),
    }
}

/// Accounts needed to initialize the reward collection account with RAA checks.
pub struct InitializeRewardCollectionAccountV1Accounts {
    pub config: Pubkey,
    pub reward_collection_account: Pubkey,
    pub rakurai_activation_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
}

/// Builds the instruction to initialize the reward collection account with RAA checks.
pub fn initialize_reward_collection_account_v1_ix(
    program_id: Pubkey,
    args: InitializeRewardCollectionAccountArgs,
    accounts: InitializeRewardCollectionAccountV1Accounts,
) -> Instruction {
    let InitializeRewardCollectionAccountArgs {
        merkle_root_upload_authority,
        block_reward_commission_bps,
        client_commission_account,
        client_commission_bps,
        bump,
    } = args;

    let InitializeRewardCollectionAccountV1Accounts {
        config,
        reward_collection_account,
        rakurai_activation_account,
        validator_vote_account,
        signer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRewardCollectionAccountV1 {
            merkle_root_upload_authority,
            block_reward_commission_bps,
            client_commission_account,
            client_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRewardCollectionAccountV1 {
            config,
            reward_collection_account,
            rakurai_activation_account,
            validator_vote_account,
            signer,
            system_program,
        }
        .to_account_metas(None),
    }
}

/// Args for closing the claim status account (empty).
pub struct CloseClaimStatusArgs;

/// Accounts required to close a claim status account.
pub struct CloseClaimStatusAccounts {
    pub config: Pubkey,
    pub claim_status: Pubkey,
    pub claim_status_payer: Pubkey,
}

/// Builds the instruction to close the claim status account.
pub fn close_claim_status_ix(
    program_id: Pubkey,
    _args: CloseClaimStatusArgs,
    accounts: CloseClaimStatusAccounts,
) -> Instruction {
    let CloseClaimStatusAccounts {
        config,
        claim_status,
        claim_status_payer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseClaimStatus {}.data(),
        accounts: crate::accounts::CloseClaimStatus {
            config,
            claim_status,
            claim_status_payer,
        }
        .to_account_metas(None),
    }
}

/// Arguments to update the reward config account.
pub struct UpdateConfigArgs {
    pub new_config: RewardDistributionConfigAccount,
}

/// Accounts needed to update the config.
pub struct UpdateConfigAccounts {
    pub config: Pubkey,
    pub authority: Pubkey,
}

/// Builds the instruction to update the reward distribution config.
pub fn update_config_ix(
    program_id: Pubkey,
    args: UpdateConfigArgs,
    accounts: UpdateConfigAccounts,
) -> Instruction {
    let UpdateConfigArgs { new_config } = args;

    let UpdateConfigAccounts { config, authority } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateConfig { new_config }.data(),
        accounts: crate::accounts::UpdateConfig { config, authority }.to_account_metas(None),
    }
}

/// Arguments for closing the reward distribution config account.
pub struct CloseConfigArgs;

/// Accounts required to close the reward distribution config account.
pub struct CloseConfigAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to close the reward distribution config account.
pub fn close_config_ix(
    program_id: Pubkey,
    _args: CloseConfigArgs,
    accounts: CloseConfigAccounts,
) -> Instruction {
    let CloseConfigAccounts { config, signer } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseConfig {}.data(),
        accounts: crate::accounts::CloseConfig { config, signer }.to_account_metas(None),
    }
}

pub struct InitializeTipsAndMevShareConfigArgs {
    pub authority: Pubkey,
    pub tip_manager_authority: Pubkey,
    pub tip_commission_account: Pubkey,
    pub tip_commission_bps: u16,
    pub tip_epoch: u8,
    pub mev_share_manager_authority: Pubkey,
    pub mev_share_commission_account: Pubkey,
    pub mev_share_commission_bps: u16,
    pub mev_share_epoch: u8,
    pub bump: u8,
}

pub struct InitializeTipsAndMevShareConfigAccounts {
    pub tips_and_mev_share_config: Pubkey,
    pub system_program: Pubkey,
    pub initializer: Pubkey,
}

pub fn initialize_tips_and_mev_share_config_ix(
    program_id: Pubkey,
    args: InitializeTipsAndMevShareConfigArgs,
    accounts: InitializeTipsAndMevShareConfigAccounts,
) -> Instruction {
    let InitializeTipsAndMevShareConfigArgs {
        authority,
        tip_manager_authority,
        tip_commission_account,
        tip_commission_bps,
        tip_epoch,
        mev_share_manager_authority,
        mev_share_commission_account,
        mev_share_commission_bps,
        mev_share_epoch,
        bump,
    } = args;
    let InitializeTipsAndMevShareConfigAccounts {
        tips_and_mev_share_config,
        system_program,
        initializer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeTipsAndMevShareConfig {
            authority,
            tip_manager_authority,
            tip_commission_account,
            tip_commission_bps,
            tip_epoch,
            mev_share_manager_authority,
            mev_share_commission_account,
            mev_share_commission_bps,
            mev_share_epoch,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeTipsAndMevShareConfig {
            tips_and_mev_share_config,
            system_program,
            initializer,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateTipsAndMevShareConfigArgs {
    pub tip_manager_authority: Pubkey,
    pub tip_commission_account: Pubkey,
    pub tip_commission_bps: u16,
    pub tip_epoch: u8,
    pub mev_share_manager_authority: Pubkey,
    pub mev_share_commission_account: Pubkey,
    pub mev_share_commission_bps: u16,
    pub mev_share_epoch: u8,
}

pub struct UpdateTipsAndMevShareConfigAccounts {
    pub tips_and_mev_share_config: Pubkey,
    pub authority: Pubkey,
}

pub fn update_tips_and_mev_share_config_ix(
    program_id: Pubkey,
    args: UpdateTipsAndMevShareConfigArgs,
    accounts: UpdateTipsAndMevShareConfigAccounts,
) -> Instruction {
    let UpdateTipsAndMevShareConfigArgs {
        tip_manager_authority,
        tip_commission_account,
        tip_commission_bps,
        tip_epoch,
        mev_share_manager_authority,
        mev_share_commission_account,
        mev_share_commission_bps,
        mev_share_epoch,
    } = args;
    let UpdateTipsAndMevShareConfigAccounts {
        tips_and_mev_share_config,
        authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateTipsAndMevShareConfig {
            tip_manager_authority,
            tip_commission_account,
            tip_commission_bps,
            tip_epoch,
            mev_share_manager_authority,
            mev_share_commission_account,
            mev_share_commission_bps,
            mev_share_epoch,
        }
        .data(),
        accounts: crate::accounts::UpdateTipsAndMevShareConfig {
            tips_and_mev_share_config,
            authority,
        }
        .to_account_metas(None),
    }
}

pub struct CloseTipsAndMevShareConfigArgs;

pub struct CloseTipsAndMevShareConfigAccounts {
    pub tips_and_mev_share_config: Pubkey,
    pub signer: Pubkey,
}

pub fn close_tips_and_mev_share_config_ix(
    program_id: Pubkey,
    _args: CloseTipsAndMevShareConfigArgs,
    accounts: CloseTipsAndMevShareConfigAccounts,
) -> Instruction {
    let CloseTipsAndMevShareConfigAccounts {
        tips_and_mev_share_config,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseTipsAndMevShareConfig {}.data(),
        accounts: crate::accounts::CloseTipsAndMevShareConfig {
            tips_and_mev_share_config,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Merkle root and claim limits for uploading new rewards.
pub struct UploadMerkleRootArgs {
    pub root: [u8; 32],
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}

/// Accounts for uploading a Merkle root to the reward collection account.
pub struct UploadMerkleRootAccounts {
    pub config: Pubkey,
    pub merkle_root_upload_authority: Pubkey,
    pub reward_collection_account: Pubkey,
}

/// Builds the instruction to upload a Merkle root.
pub fn upload_merkle_root_ix(
    program_id: Pubkey,
    args: UploadMerkleRootArgs,
    accounts: UploadMerkleRootAccounts,
) -> Instruction {
    let UploadMerkleRootArgs {
        root,
        max_total_claim,
        max_num_nodes,
    } = args;

    let UploadMerkleRootAccounts {
        config,
        merkle_root_upload_authority,
        reward_collection_account,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UploadMerkleRoot {
            max_total_claim,
            max_num_nodes,
            root,
        }
        .data(),
        accounts: crate::accounts::UploadMerkleRoot {
            config,
            merkle_root_upload_authority,
            reward_collection_account,
        }
        .to_account_metas(None),
    }
}

/// Total amount to be transferred to stakers.
pub struct TransferStakerRewardsArgs {
    pub total_rewards: u64,
}

/// Accounts required to transfer rewards to stakers.
pub struct TransferStakerRewardsAccounts {
    pub client_commission_account: Pubkey,
    pub reward_collection_account: Pubkey,
    pub system_program: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to transfer staker rewards.
pub fn transfer_staker_rewards_ix(
    program_id: Pubkey,
    args: TransferStakerRewardsArgs,
    accounts: TransferStakerRewardsAccounts,
) -> Instruction {
    let TransferStakerRewardsArgs { total_rewards } = args;

    let TransferStakerRewardsAccounts {
        client_commission_account,
        reward_collection_account,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::TransferStakerRewards { total_rewards }.data(),
        accounts: crate::accounts::TransferStakerRewards {
            client_commission_account,
            reward_collection_account,
            system_program,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Total MEV rewards earned by the validator in the epoch (if MEV commission is set by validator in TipDistributionAccount).
pub struct TransferClientCommissionOnMevCommissionArgs {
    pub mev_rewards: u64,
}

/// Accounts required to transfer MEV commission to the client commission account.
pub struct TransferClientCommissionOnMevCommissionAccounts {
    pub client_commission_account: Pubkey,
    pub reward_collection_account: Pubkey,
    pub system_program: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to deduct client commission from the validator’s MEV rewards.
pub fn transfer_client_commission_on_mev_commission_ix(
    program_id: Pubkey,
    args: TransferClientCommissionOnMevCommissionArgs,
    accounts: TransferClientCommissionOnMevCommissionAccounts,
) -> Instruction {
    let TransferClientCommissionOnMevCommissionArgs { mev_rewards } = args;

    let TransferClientCommissionOnMevCommissionAccounts {
        client_commission_account,
        reward_collection_account,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::TransferClientCommissionOnMevCommission { mev_rewards }
            .data(),
        accounts: crate::accounts::TransferClientCommissionOnMevCommission {
            client_commission_account,
            reward_collection_account,
            system_program,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Epoch argument (for context) when closing the reward collection account.
pub struct CloseRewardCollectionAccountArgs {
    pub _epoch: u64,
}

/// Accounts needed to close the reward collection account.
pub struct CloseRewardCollectionAccounts {
    pub config: Pubkey,
    pub initializer: Pubkey,
    pub reward_collection_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to close the reward collection account.
pub fn close_reward_collection_account_ix(
    program_id: Pubkey,
    args: CloseRewardCollectionAccountArgs,
    accounts: CloseRewardCollectionAccounts,
) -> Instruction {
    let CloseRewardCollectionAccountArgs { _epoch } = args;

    let CloseRewardCollectionAccounts {
        config,
        initializer,
        reward_collection_account,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRewardCollectionAccount { _epoch }.data(),
        accounts: crate::accounts::CloseRewardCollectionAccount {
            config,
            initializer,
            validator_vote_account,
            reward_collection_account,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Proof and metadata for a Merkle claim.
pub struct ClaimArgs {
    pub proof: Vec<[u8; 32]>,
    pub amount: u64,
    pub bump: u8,
}

/// Accounts needed to execute a Merkle reward claim.
pub struct ClaimAccounts {
    pub reward_collection_account: Pubkey,
    pub claim_status: Pubkey,
    pub claimant: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

/// Builds the instruction for claiming Merkle rewards.
pub fn claim_ix(program_id: Pubkey, args: ClaimArgs, accounts: ClaimAccounts) -> Instruction {
    let ClaimArgs {
        proof,
        amount,
        bump,
    } = args;

    let ClaimAccounts {
        reward_collection_account,
        claim_status,
        claimant,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::Claim {
            proof,
            amount,
            bump,
        }
        .data(),
        accounts: crate::accounts::Claim {
            reward_collection_account,
            claimant,
            claim_status,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct InitializeRevenueShareAccountArgs {
    pub share_kind: RevenueKind,
    pub name: [u8; 32],
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub bump: u8,
}

pub struct InitializeRevenueShareAccountAccounts {
    pub revenue_share_account: Pubkey,
    pub config: Pubkey,
    pub rakurai_activation_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

pub fn initialize_revenue_share_account_ix(
    program_id: Pubkey,
    args: InitializeRevenueShareAccountArgs,
    accounts: InitializeRevenueShareAccountAccounts,
) -> Instruction {
    let InitializeRevenueShareAccountArgs {
        share_kind,
        name,
        record_authority,
        max_epoch_entries,
        commission_bps,
        commission_account,
        bump,
    } = args;
    let InitializeRevenueShareAccountAccounts {
        revenue_share_account,
        config,
        rakurai_activation_account,
        validator_vote_account,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRevenueShareAccount {
            share_kind,
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRevenueShareAccount {
            revenue_share_account,
            config,
            rakurai_activation_account,
            validator_vote_account,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct InitializeRevenueShareAccountV1Args {
    pub share_kind: RevenueKind,
    pub name: [u8; 32],
    pub record_authority: Pubkey,
    pub bump: u8,
}

pub struct InitializeRevenueShareAccountV1Accounts {
    pub revenue_share_account: Pubkey,
    pub tips_and_mev_share_config: Pubkey,
    pub rakurai_activation_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

pub fn initialize_revenue_share_account_v1_ix(
    program_id: Pubkey,
    args: InitializeRevenueShareAccountV1Args,
    accounts: InitializeRevenueShareAccountV1Accounts,
) -> Instruction {
    let InitializeRevenueShareAccountV1Args {
        share_kind,
        name,
        record_authority,
        bump,
    } = args;
    let InitializeRevenueShareAccountV1Accounts {
        revenue_share_account,
        tips_and_mev_share_config,
        rakurai_activation_account,
        validator_vote_account,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRevenueShareAccountV1 {
            share_kind,
            name,
            record_authority,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRevenueShareAccountV1 {
            revenue_share_account,
            tips_and_mev_share_config,
            rakurai_activation_account,
            validator_vote_account,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct RecordRevenueArgs {
    pub amount: u64,
}

pub struct RecordRevenueShareAccounts {
    pub revenue_share_account: Pubkey,
    pub record_authority: Pubkey,
}

pub fn record_revenue_ix(
    program_id: Pubkey,
    args: RecordRevenueArgs,
    accounts: RecordRevenueShareAccounts,
) -> Instruction {
    let RecordRevenueArgs { amount } = args;
    let RecordRevenueShareAccounts {
        revenue_share_account,
        record_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RecordRevenue { amount }.data(),
        accounts: crate::accounts::RecordRevenue {
            revenue_share_account,
            record_authority,
        }
        .to_account_metas(None),
    }
}

pub fn record_revenue_v1_ix(
    program_id: Pubkey,
    args: RecordRevenueArgs,
    accounts: RecordRevenueShareAccounts,
) -> Instruction {
    let RecordRevenueArgs { amount } = args;
    let RecordRevenueShareAccounts {
        revenue_share_account,
        record_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RecordRevenueV1 { amount }.data(),
        accounts: crate::accounts::RecordRevenueV1 {
            revenue_share_account,
            record_authority,
        }
        .to_account_metas(None),
    }
}

pub struct SettleRevenueArgs {
    pub epoch: u64,
    pub amount: u64,
}

pub struct SettleRevenueAccounts {
    pub revenue_share_account: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

pub fn settle_revenue_ix(
    program_id: Pubkey,
    args: SettleRevenueArgs,
    accounts: SettleRevenueAccounts,
) -> Instruction {
    let SettleRevenueArgs { epoch, amount } = args;
    let SettleRevenueAccounts {
        revenue_share_account,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::SettleRevenue { epoch, amount }.data(),
        accounts: crate::accounts::SettleRevenue {
            revenue_share_account,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateTransferredAmountArgs {
    pub epoch: u64,
    pub amount: u64,
}

pub struct UpdateTransferredAmountAccounts {
    pub revenue_share_account: Pubkey,
    pub authority: Pubkey,
}

pub fn update_transferred_amount_ix(
    program_id: Pubkey,
    args: UpdateTransferredAmountArgs,
    accounts: UpdateTransferredAmountAccounts,
) -> Instruction {
    let UpdateTransferredAmountArgs { epoch, amount } = args;
    let UpdateTransferredAmountAccounts {
        revenue_share_account,
        authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateTransferredAmount { epoch, amount }.data(),
        accounts: crate::accounts::UpdateTransferredAmount {
            revenue_share_account,
            authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateDeficitArgs {
    pub update: crate::state::DeficitUpdate,
}

pub struct UpdateDeficitAccounts {
    pub revenue_share_account: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_deficit_ix(
    program_id: Pubkey,
    args: UpdateDeficitArgs,
    accounts: UpdateDeficitAccounts,
) -> Instruction {
    let UpdateDeficitArgs { update } = args;
    let UpdateDeficitAccounts {
        revenue_share_account,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateDeficit { update }.data(),
        accounts: crate::accounts::UpdateDeficit {
            revenue_share_account,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimRevenueArgs {
    pub epoch: u64,
}

pub struct ClaimRevenueShareAccounts {
    pub revenue_share_account: Pubkey,
    pub commission_account: Pubkey,
    pub validator_identity: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn claim_revenue_ix(
    program_id: Pubkey,
    args: ClaimRevenueArgs,
    accounts: ClaimRevenueShareAccounts,
) -> Instruction {
    let ClaimRevenueArgs { epoch } = args;
    let ClaimRevenueShareAccounts {
        revenue_share_account,
        commission_account,
        validator_identity,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimRevenue { epoch }.data(),
        accounts: crate::accounts::ClaimRevenue {
            revenue_share_account,
            commission_account,
            validator_identity,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub fn claim_revenue_v1_ix(
    program_id: Pubkey,
    args: ClaimRevenueArgs,
    accounts: ClaimRevenueShareAccounts,
) -> Instruction {
    let ClaimRevenueArgs { epoch } = args;
    let ClaimRevenueShareAccounts {
        revenue_share_account,
        commission_account,
        validator_identity,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimRevenueV1 { epoch }.data(),
        accounts: crate::accounts::ClaimRevenueV1 {
            revenue_share_account,
            commission_account,
            validator_identity,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateRevenueShareConfigArgs {
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub block_reward_conversion_enabled: bool,
    pub record_authority: Option<Pubkey>,
}

pub struct UpdateRevenueShareConfigAccounts {
    pub revenue_share_account: Pubkey,
    pub config: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_revenue_share_config_ix(
    program_id: Pubkey,
    args: UpdateRevenueShareConfigArgs,
    accounts: UpdateRevenueShareConfigAccounts,
) -> Instruction {
    let UpdateRevenueShareConfigArgs {
        commission_bps,
        commission_account,
        block_reward_conversion_enabled,
        record_authority,
    } = args;
    let UpdateRevenueShareConfigAccounts {
        revenue_share_account,
        config,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRevenueShareConfig {
            commission_bps,
            commission_account,
            block_reward_conversion_enabled,
            record_authority,
        }
        .data(),
        accounts: crate::accounts::UpdateRevenueShareConfig {
            revenue_share_account,
            config,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

/// Updates TCAV1 / MCAV1 config (`REVENUE_SHARE_V1`).
pub fn update_revenue_share_config_v1_ix(
    program_id: Pubkey,
    args: UpdateRevenueShareConfigArgs,
    accounts: UpdateRevenueShareConfigAccounts,
) -> Instruction {
    let UpdateRevenueShareConfigArgs {
        commission_bps,
        commission_account,
        block_reward_conversion_enabled,
        record_authority,
    } = args;
    let UpdateRevenueShareConfigAccounts {
        revenue_share_account,
        config,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRevenueShareConfigV1 {
            commission_bps,
            commission_account,
            block_reward_conversion_enabled,
            record_authority,
        }
        .data(),
        accounts: crate::accounts::UpdateRevenueShareConfigV1 {
            revenue_share_account,
            config,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateEpochConvertedToBlockRewardArgs {
    pub epoch: u64,
}

pub struct UpdateEpochConvertedToBlockRewardAccounts {
    pub revenue_share_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
}

pub fn update_epoch_converted_to_block_reward_ix(
    program_id: Pubkey,
    args: UpdateEpochConvertedToBlockRewardArgs,
    accounts: UpdateEpochConvertedToBlockRewardAccounts,
) -> Instruction {
    let UpdateEpochConvertedToBlockRewardArgs { epoch } = args;
    let UpdateEpochConvertedToBlockRewardAccounts {
        revenue_share_account,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateEpochConvertedToBlockReward { epoch }.data(),
        accounts: crate::accounts::UpdateEpochConvertedToBlockReward {
            revenue_share_account,
            validator_vote_account,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Marks a claimed TCAV1 / MCAV1 epoch as `block_reward_converted`.
pub fn update_epoch_converted_to_block_reward_v1_ix(
    program_id: Pubkey,
    args: UpdateEpochConvertedToBlockRewardArgs,
    accounts: UpdateEpochConvertedToBlockRewardAccounts,
) -> Instruction {
    let UpdateEpochConvertedToBlockRewardArgs { epoch } = args;
    let UpdateEpochConvertedToBlockRewardAccounts {
        revenue_share_account,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateEpochConvertedToBlockRewardV1 { epoch }.data(),
        accounts: crate::accounts::UpdateEpochConvertedToBlockRewardV1 {
            revenue_share_account,
            validator_vote_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct CloseRevenueShareAccountAccounts {
    pub revenue_share_account: Pubkey,
    pub initializer: Pubkey,
    pub authority: Pubkey,
}

pub fn close_revenue_share_account_ix(
    program_id: Pubkey,
    accounts: CloseRevenueShareAccountAccounts,
) -> Instruction {
    let CloseRevenueShareAccountAccounts {
        revenue_share_account,
        initializer,
        authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRevenueShareAccount {}.data(),
        accounts: crate::accounts::CloseRevenueShareAccount {
            revenue_share_account,
            initializer,
            authority,
        }
        .to_account_metas(None),
    }
}

pub struct CloseRevenueShareAccountV1Accounts {
    pub revenue_share_account: Pubkey,
    pub initializer: Pubkey,
    pub authority: Pubkey,
}

pub fn close_revenue_share_account_v1_ix(
    program_id: Pubkey,
    accounts: CloseRevenueShareAccountV1Accounts,
) -> Instruction {
    let CloseRevenueShareAccountV1Accounts {
        revenue_share_account,
        initializer,
        authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRevenueShareAccountV1 {}.data(),
        accounts: crate::accounts::CloseRevenueShareAccountV1 {
            revenue_share_account,
            initializer,
            authority,
        }
        .to_account_metas(None),
    }
}

// ---- P2C subscription ----

pub struct InitializeP2CSubscriptionAccountArgs {
    pub name: [u8; 32],
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub grace_epochs: u8,
    pub bump: u8,
}

pub struct InitializeP2CSubscriptionAccountAccounts {
    pub p2c_subscription_account: Pubkey,
    pub config: Pubkey,
    pub validator_vote_account: Pubkey,
    pub manager_authority: Pubkey,
    pub system_program: Pubkey,
}

pub fn initialize_p2c_subscription_account_ix(
    program_id: Pubkey,
    args: InitializeP2CSubscriptionAccountArgs,
    accounts: InitializeP2CSubscriptionAccountAccounts,
) -> Instruction {
    let InitializeP2CSubscriptionAccountArgs {
        name,
        record_authority,
        max_epoch_entries,
        commission_bps,
        commission_account,
        grace_epochs,
        bump,
    } = args;
    let InitializeP2CSubscriptionAccountAccounts {
        p2c_subscription_account,
        config,
        validator_vote_account,
        manager_authority,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeP2cSubscriptionAccount {
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            grace_epochs,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeP2CSubscriptionAccount {
            p2c_subscription_account,
            config,
            validator_vote_account,
            manager_authority,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct FundP2CSubscriptionArgs {
    pub amount: u64,
}

pub struct FundP2CSubscriptionAccounts {
    pub p2c_subscription_account: Pubkey,
    pub funder: Pubkey,
    pub system_program: Pubkey,
}

pub fn fund_p2c_subscription_ix(
    program_id: Pubkey,
    args: FundP2CSubscriptionArgs,
    accounts: FundP2CSubscriptionAccounts,
) -> Instruction {
    let FundP2CSubscriptionArgs { amount } = args;
    let FundP2CSubscriptionAccounts {
        p2c_subscription_account,
        funder,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::FundP2cSubscription { amount }.data(),
        accounts: crate::accounts::FundP2CSubscription {
            p2c_subscription_account,
            funder,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct RecordP2CSubscriptionArgs {
    pub epoch: u64,
    pub stake: u64,
    pub amount_due: u64,
}

pub struct RecordP2CSubscriptionAccounts {
    pub p2c_subscription_account: Pubkey,
    pub record_authority: Pubkey,
}

pub fn record_p2c_subscription_ix(
    program_id: Pubkey,
    args: RecordP2CSubscriptionArgs,
    accounts: RecordP2CSubscriptionAccounts,
) -> Instruction {
    let RecordP2CSubscriptionArgs {
        epoch,
        stake,
        amount_due,
    } = args;
    let RecordP2CSubscriptionAccounts {
        p2c_subscription_account,
        record_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RecordP2cSubscription {
            epoch,
            stake,
            amount_due,
        }
        .data(),
        accounts: crate::accounts::RecordP2CSubscription {
            p2c_subscription_account,
            record_authority,
        }
        .to_account_metas(None),
    }
}

pub struct DeductP2CSubscriptionArgs {
    pub epoch: u64,
}

pub struct DeductP2CSubscriptionAccounts {
    pub p2c_subscription_account: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn deduct_p2c_subscription_ix(
    program_id: Pubkey,
    args: DeductP2CSubscriptionArgs,
    accounts: DeductP2CSubscriptionAccounts,
) -> Instruction {
    let DeductP2CSubscriptionArgs { epoch } = args;
    let DeductP2CSubscriptionAccounts {
        p2c_subscription_account,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::DeductP2cSubscription { epoch }.data(),
        accounts: crate::accounts::DeductP2CSubscription {
            p2c_subscription_account,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimP2CSubscriptionArgs {
    pub epoch: u64,
}

pub struct ClaimP2CSubscriptionAccounts {
    pub p2c_subscription_account: Pubkey,
    pub commission_account: Pubkey,
    pub validator_identity: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn claim_p2c_subscription_ix(
    program_id: Pubkey,
    args: ClaimP2CSubscriptionArgs,
    accounts: ClaimP2CSubscriptionAccounts,
) -> Instruction {
    let ClaimP2CSubscriptionArgs { epoch } = args;
    let ClaimP2CSubscriptionAccounts {
        p2c_subscription_account,
        commission_account,
        validator_identity,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimP2cSubscription { epoch }.data(),
        accounts: crate::accounts::ClaimP2CSubscription {
            p2c_subscription_account,
            commission_account,
            validator_identity,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateP2CEpochConvertedToBlockRewardArgs {
    pub epoch: u64,
}

pub struct UpdateP2CEpochConvertedToBlockRewardAccounts {
    pub p2c_subscription_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
}

pub fn update_p2c_epoch_converted_to_block_reward_ix(
    program_id: Pubkey,
    args: UpdateP2CEpochConvertedToBlockRewardArgs,
    accounts: UpdateP2CEpochConvertedToBlockRewardAccounts,
) -> Instruction {
    let UpdateP2CEpochConvertedToBlockRewardArgs { epoch } = args;
    let UpdateP2CEpochConvertedToBlockRewardAccounts {
        p2c_subscription_account,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateP2cEpochConvertedToBlockReward { epoch }.data(),
        accounts: crate::accounts::UpdateP2CEpochConvertedToBlockReward {
            p2c_subscription_account,
            validator_vote_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateP2CSubscriptionConfigArgs {
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub block_reward_conversion_enabled: bool,
    pub grace_epochs: Option<u8>,
    pub record_authority: Option<Pubkey>,
}

pub struct UpdateP2CSubscriptionConfigAccounts {
    pub p2c_subscription_account: Pubkey,
    pub config: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_p2c_subscription_config_ix(
    program_id: Pubkey,
    args: UpdateP2CSubscriptionConfigArgs,
    accounts: UpdateP2CSubscriptionConfigAccounts,
) -> Instruction {
    let UpdateP2CSubscriptionConfigArgs {
        commission_bps,
        commission_account,
        block_reward_conversion_enabled,
        grace_epochs,
        record_authority,
    } = args;
    let UpdateP2CSubscriptionConfigAccounts {
        p2c_subscription_account,
        config,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateP2cSubscriptionConfig {
            commission_bps,
            commission_account,
            block_reward_conversion_enabled,
            grace_epochs,
            record_authority,
        }
        .data(),
        accounts: crate::accounts::UpdateP2CSubscriptionConfig {
            p2c_subscription_account,
            config,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateP2CDeficitArgs {
    pub update: crate::state::DeficitUpdate,
}

pub struct UpdateP2CDeficitAccounts {
    pub p2c_subscription_account: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_p2c_deficit_ix(
    program_id: Pubkey,
    args: UpdateP2CDeficitArgs,
    accounts: UpdateP2CDeficitAccounts,
) -> Instruction {
    let UpdateP2CDeficitArgs { update } = args;
    let UpdateP2CDeficitAccounts {
        p2c_subscription_account,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateP2cDeficit { update }.data(),
        accounts: crate::accounts::UpdateP2CDeficit {
            p2c_subscription_account,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct CloseP2CSubscriptionAccountAccounts {
    pub p2c_subscription_account: Pubkey,
    pub initializer: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn close_p2c_subscription_account_ix(
    program_id: Pubkey,
    accounts: CloseP2CSubscriptionAccountAccounts,
) -> Instruction {
    let CloseP2CSubscriptionAccountAccounts {
        p2c_subscription_account,
        initializer,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseP2cSubscriptionAccount {}.data(),
        accounts: crate::accounts::CloseP2CSubscriptionAccount {
            p2c_subscription_account,
            initializer,
            manager_authority,
        }
        .to_account_metas(None),
    }
}
