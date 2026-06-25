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
    pub block_builder_commission_on_mev_commission_enabled: bool,
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
        block_builder_commission_on_mev_commission_enabled,
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
            block_builder_commission_on_mev_commission_enabled,
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
    pub block_builder_commission_account: Pubkey,
    pub block_builder_commission_bps: u16,
    pub bump: u8,
}

/// Accounts needed to initialize the reward collection account.
pub struct InitializeRewardCollectionAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub reward_collection_account: Pubkey,
    pub rakurai_activation_account: Pubkey,
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
        block_reward_commission_bps,
        block_builder_commission_account,
        block_builder_commission_bps,
        bump,
    } = args;

    let InitializeRewardCollectionAccountAccounts {
        config,
        reward_collection_account,
        rakurai_activation_account,
        system_program,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRewardCollectionAccount {
            merkle_root_upload_authority,
            block_reward_commission_bps,
            block_builder_commission_account,
            block_builder_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRewardCollectionAccount {
            config,
            signer,
            system_program,
            reward_collection_account,
            rakurai_activation_account,
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
    pub block_builder_commission_account: Pubkey,
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
        block_builder_commission_account,
        reward_collection_account,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::TransferStakerRewards { total_rewards }.data(),
        accounts: crate::accounts::TransferStakerRewards {
            block_builder_commission_account,
            reward_collection_account,
            system_program,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Total MEV rewards earned by the validator in the epoch (if MEV commission is set by validator in TipDistributionAccount).
pub struct TransferBlockBuilderCommissionOnMevCommissionArgs {
    pub mev_rewards: u64,
}

/// Accounts required to transfer MEV commission to the block builder commission account.
pub struct TransferBlockBuilderCommissionOnMevCommissionAccounts {
    pub block_builder_commission_account: Pubkey,
    pub reward_collection_account: Pubkey,
    pub system_program: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to deduct block builder commission from the validator’s MEV rewards.
pub fn transfer_block_builder_commission_on_mev_commission_ix(
    program_id: Pubkey,
    args: TransferBlockBuilderCommissionOnMevCommissionArgs,
    accounts: TransferBlockBuilderCommissionOnMevCommissionAccounts,
) -> Instruction {
    let TransferBlockBuilderCommissionOnMevCommissionArgs { mev_rewards } = args;

    let TransferBlockBuilderCommissionOnMevCommissionAccounts {
        block_builder_commission_account,
        reward_collection_account,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::TransferBlockBuilderCommissionOnMevCommission { mev_rewards }
            .data(),
        accounts: crate::accounts::TransferBlockBuilderCommissionOnMevCommission {
            block_builder_commission_account,
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

pub struct InitializePartnerTipShareAccountArgs {
    pub name: [u8; 32],
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub bump: u8,
}

pub struct InitializePartnerTipShareAccountAccounts {
    pub partner_tip_share_account: Pubkey,
    pub config: Pubkey,
    pub validator_vote_account: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

pub fn initialize_partner_tip_share_account_ix(
    program_id: Pubkey,
    args: InitializePartnerTipShareAccountArgs,
    accounts: InitializePartnerTipShareAccountAccounts,
) -> Instruction {
    let InitializePartnerTipShareAccountArgs {
        name,
        record_authority,
        max_epoch_entries,
        commission_bps,
        commission_account,
        bump,
    } = args;
    let InitializePartnerTipShareAccountAccounts {
        partner_tip_share_account,
        config,
        validator_vote_account,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializePartnerTipShareAccount {
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializePartnerTipShareAccount {
            partner_tip_share_account,
            config,
            validator_vote_account,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct InitializePartnerBackrunShareAccountArgs {
    pub name: [u8; 32],
    pub record_authority: Pubkey,
    pub max_epoch_entries: u8,
    pub commission_bps: u16,
    pub commission_account: Pubkey,
    pub bump: u8,
}

pub struct InitializePartnerBackrunShareAccountAccounts {
    pub partner_backrun_share_account: Pubkey,
    pub config: Pubkey,
    pub validator_vote_account: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}

pub fn initialize_partner_backrun_share_account_ix(
    program_id: Pubkey,
    args: InitializePartnerBackrunShareAccountArgs,
    accounts: InitializePartnerBackrunShareAccountAccounts,
) -> Instruction {
    let InitializePartnerBackrunShareAccountArgs {
        name,
        record_authority,
        max_epoch_entries,
        commission_bps,
        commission_account,
        bump,
    } = args;
    let InitializePartnerBackrunShareAccountAccounts {
        partner_backrun_share_account,
        config,
        validator_vote_account,
        payer,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializePartnerBackrunShareAccount {
            name,
            record_authority,
            max_epoch_entries,
            commission_bps,
            commission_account,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializePartnerBackrunShareAccount {
            partner_backrun_share_account,
            config,
            validator_vote_account,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct RecordPartnerTipShareArgs {
    pub amount: u64,
}

pub struct RecordPartnerTipShareAccounts {
    pub partner_tip_share_account: Pubkey,
    pub record_authority: Pubkey,
}

pub fn record_partner_tip_share_ix(
    program_id: Pubkey,
    args: RecordPartnerTipShareArgs,
    accounts: RecordPartnerTipShareAccounts,
) -> Instruction {
    let RecordPartnerTipShareArgs { amount } = args;
    let RecordPartnerTipShareAccounts {
        partner_tip_share_account,
        record_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RecordPartnerTipShare { amount }.data(),
        accounts: crate::accounts::RecordPartnerTipShare {
            partner_tip_share_account,
            record_authority,
        }
        .to_account_metas(None),
    }
}

pub struct RecordPartnerBackrunShareArgs {
    pub amount: u64,
}

pub struct RecordPartnerBackrunShareAccounts {
    pub partner_backrun_share_account: Pubkey,
    pub record_authority: Pubkey,
}

pub fn record_partner_backrun_share_ix(
    program_id: Pubkey,
    args: RecordPartnerBackrunShareArgs,
    accounts: RecordPartnerBackrunShareAccounts,
) -> Instruction {
    let RecordPartnerBackrunShareArgs { amount } = args;
    let RecordPartnerBackrunShareAccounts {
        partner_backrun_share_account,
        record_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RecordPartnerBackrunShare { amount }.data(),
        accounts: crate::accounts::RecordPartnerBackrunShare {
            partner_backrun_share_account,
            record_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimPartnerTipShareArgs {
    pub epoch: u64,
}

pub struct ClaimPartnerTipShareAccounts {
    pub partner_tip_share_account: Pubkey,
    pub commission_account: Pubkey,
    pub validator_identity: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn claim_partner_tip_share_ix(
    program_id: Pubkey,
    args: ClaimPartnerTipShareArgs,
    accounts: ClaimPartnerTipShareAccounts,
) -> Instruction {
    let ClaimPartnerTipShareArgs { epoch } = args;
    let ClaimPartnerTipShareAccounts {
        partner_tip_share_account,
        commission_account,
        validator_identity,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimPartnerTipShare { epoch }.data(),
        accounts: crate::accounts::ClaimPartnerTipShare {
            partner_tip_share_account,
            commission_account,
            validator_identity,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimPartnerBackrunShareArgs {
    pub epoch: u64,
}

pub struct ClaimPartnerBackrunShareAccounts {
    pub partner_backrun_share_account: Pubkey,
    pub commission_account: Pubkey,
    pub validator_identity: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn claim_partner_backrun_share_ix(
    program_id: Pubkey,
    args: ClaimPartnerBackrunShareArgs,
    accounts: ClaimPartnerBackrunShareAccounts,
) -> Instruction {
    let ClaimPartnerBackrunShareArgs { epoch } = args;
    let ClaimPartnerBackrunShareAccounts {
        partner_backrun_share_account,
        commission_account,
        validator_identity,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimPartnerBackrunShare { epoch }.data(),
        accounts: crate::accounts::ClaimPartnerBackrunShare {
            partner_backrun_share_account,
            commission_account,
            validator_identity,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdatePartnerTipShareCommissionArgs {
    pub commission_bps: u16,
    pub commission_account: Pubkey,
}

pub struct UpdatePartnerTipShareCommissionAccounts {
    pub partner_tip_share_account: Pubkey,
    pub config: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_partner_tip_share_commission_ix(
    program_id: Pubkey,
    args: UpdatePartnerTipShareCommissionArgs,
    accounts: UpdatePartnerTipShareCommissionAccounts,
) -> Instruction {
    let UpdatePartnerTipShareCommissionArgs {
        commission_bps,
        commission_account,
    } = args;
    let UpdatePartnerTipShareCommissionAccounts {
        partner_tip_share_account,
        config,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdatePartnerTipShareCommission {
            commission_bps,
            commission_account,
        }
        .data(),
        accounts: crate::accounts::UpdatePartnerTipShareCommission {
            partner_tip_share_account,
            config,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct UpdatePartnerBackrunShareCommissionArgs {
    pub commission_bps: u16,
    pub commission_account: Pubkey,
}

pub struct UpdatePartnerBackrunShareCommissionAccounts {
    pub partner_backrun_share_account: Pubkey,
    pub config: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn update_partner_backrun_share_commission_ix(
    program_id: Pubkey,
    args: UpdatePartnerBackrunShareCommissionArgs,
    accounts: UpdatePartnerBackrunShareCommissionAccounts,
) -> Instruction {
    let UpdatePartnerBackrunShareCommissionArgs {
        commission_bps,
        commission_account,
    } = args;
    let UpdatePartnerBackrunShareCommissionAccounts {
        partner_backrun_share_account,
        config,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdatePartnerBackrunShareCommission {
            commission_bps,
            commission_account,
        }
        .data(),
        accounts: crate::accounts::UpdatePartnerBackrunShareCommission {
            partner_backrun_share_account,
            config,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClosePartnerTipShareAccountAccounts {
    pub partner_tip_share_account: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn close_partner_tip_share_account_ix(
    program_id: Pubkey,
    accounts: ClosePartnerTipShareAccountAccounts,
) -> Instruction {
    let ClosePartnerTipShareAccountAccounts {
        partner_tip_share_account,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClosePartnerTipShareAccount {}.data(),
        accounts: crate::accounts::ClosePartnerTipShareAccount {
            partner_tip_share_account,
            manager_authority,
        }
        .to_account_metas(None),
    }
}

pub struct ClosePartnerBackrunShareAccountAccounts {
    pub partner_backrun_share_account: Pubkey,
    pub manager_authority: Pubkey,
}

pub fn close_partner_backrun_share_account_ix(
    program_id: Pubkey,
    accounts: ClosePartnerBackrunShareAccountAccounts,
) -> Instruction {
    let ClosePartnerBackrunShareAccountAccounts {
        partner_backrun_share_account,
        manager_authority,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClosePartnerBackrunShareAccount {}.data(),
        accounts: crate::accounts::ClosePartnerBackrunShareAccount {
            partner_backrun_share_account,
            manager_authority,
        }
        .to_account_metas(None),
    }
}
