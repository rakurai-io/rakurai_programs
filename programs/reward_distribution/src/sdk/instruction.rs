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

pub struct UpdateRevenueShareConfigArgs {
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub convert_to_block_rewards: bool,
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
        convert_to_block_rewards,
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
            convert_to_block_rewards,
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
