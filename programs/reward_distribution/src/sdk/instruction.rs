//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::RewardDistributionConfigAccount;

/// Arguments for initializing the reward distribution config account.
pub struct InitializeArgs {
    pub authority: Pubkey,
    pub num_epochs_valid: u64,
    pub max_commission_bps: u16,
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
    pub validator_commission_bps: u16,
    pub rakurai_commission_account: Pubkey,
    pub rakurai_commission_bps: u16,
    pub bump: u8,
}

/// Accounts needed to initialize the reward collection account.
pub struct InitializeRewardCollectionAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub reward_collection_account: Pubkey,
    pub validator_vote_account: Pubkey,
}

/// Builds the instruction to initialize the reward collection account.
pub fn initialize_reward_collection_account_ix(
    program_id: Pubkey,
    args: InitializeRewardCollectionAccountArgs,
    accounts: InitializeRewardCollectionAccountAccounts,
) -> Instruction {
    let InitializeRewardCollectionAccountArgs {
        merkle_root_upload_authority,
        validator_commission_bps,
        rakurai_commission_account,
        rakurai_commission_bps,
        bump,
    } = args;

    let InitializeRewardCollectionAccountAccounts {
        config,
        reward_collection_account,
        system_program,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRewardCollectionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            rakurai_commission_account,
            rakurai_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRewardCollectionAccount {
            config,
            signer,
            system_program,
            reward_collection_account,
            validator_vote_account,
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
    new_config: RewardDistributionConfigAccount,
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
    pub rakurai_commission_account: Pubkey,
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
        rakurai_commission_account,
        reward_collection_account,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::TransferStakerRewards { total_rewards }.data(),
        accounts: crate::accounts::TransferStakerRewards {
            rakurai_commission_account,
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
    pub config: Pubkey,
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
        config,
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
            config,
            reward_collection_account,
            claimant,
            claim_status,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}
