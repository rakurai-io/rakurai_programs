//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::Config;

pub struct InitializeArgs {
    pub authority: Pubkey,
    pub num_epochs_valid: u64,
    pub max_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeAccounts {
    pub config: Pubkey,
    pub system_program: Pubkey,
    pub initializer: Pubkey,
}
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

pub struct InitializeRewardDistributionAccountArgs {
    pub merkle_root_upload_authority: Pubkey,
    pub validator_commission_bps: u16,
    pub rakurai_commission_pubkey: Pubkey,
    pub rakurai_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeRewardDistributionAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub reward_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
}
pub fn initialize_reward_distribution_account_ix(
    program_id: Pubkey,
    args: InitializeRewardDistributionAccountArgs,
    accounts: InitializeRewardDistributionAccountAccounts,
) -> Instruction {
    let InitializeRewardDistributionAccountArgs {
        merkle_root_upload_authority,
        validator_commission_bps,
        rakurai_commission_pubkey,
        rakurai_commission_bps,
        bump,
    } = args;

    let InitializeRewardDistributionAccountAccounts {
        config,
        reward_distribution_account,
        system_program,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRewardDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            rakurai_commission_pubkey,
            rakurai_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRewardDistributionAccount {
            config,
            signer,
            system_program,
            reward_distribution_account,
            validator_vote_account,
        }
        .to_account_metas(None),
    }
}

pub struct CloseClaimStatusArgs;
pub struct CloseClaimStatusAccounts {
    pub config: Pubkey,
    pub claim_status: Pubkey,
    pub claim_status_payer: Pubkey,
}
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

pub struct UpdateConfigArgs {
    new_config: Config,
}
pub struct UpdateConfigAccounts {
    pub config: Pubkey,
    pub authority: Pubkey,
}
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

pub struct UploadMerkleRootArgs {
    pub root: [u8; 32],
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
}
pub struct UploadMerkleRootAccounts {
    pub config: Pubkey,
    pub merkle_root_upload_authority: Pubkey,
    pub reward_distribution_account: Pubkey,
}
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
        reward_distribution_account,
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
            reward_distribution_account,
        }
        .to_account_metas(None),
    }
}

pub struct CloseRewardDistributionAccountArgs {
    pub _epoch: u64,
}
pub struct CloseRewardDistributionAccounts {
    pub config: Pubkey,
    pub reward_distribution_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub expired_funds_account: Pubkey,
    pub signer: Pubkey,
}
pub fn close_reward_distribution_account_ix(
    program_id: Pubkey,
    args: CloseRewardDistributionAccountArgs,
    accounts: CloseRewardDistributionAccounts,
) -> Instruction {
    let CloseRewardDistributionAccountArgs { _epoch } = args;

    let CloseRewardDistributionAccounts {
        config,
        reward_distribution_account,
        validator_vote_account,
        expired_funds_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRewardDistributionAccount { _epoch }.data(),
        accounts: crate::accounts::CloseRewardDistributionAccount {
            config,
            validator_vote_account,
            expired_funds_account,
            reward_distribution_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct ClaimArgs {
    pub proof: Vec<[u8; 32]>,
    pub amount: u64,
    pub bump: u8,
}
pub struct ClaimAccounts {
    pub config: Pubkey,
    pub reward_distribution_account: Pubkey,
    pub claim_status: Pubkey,
    pub claimant: Pubkey,
    pub payer: Pubkey,
    pub system_program: Pubkey,
}
pub fn claim_ix(program_id: Pubkey, args: ClaimArgs, accounts: ClaimAccounts) -> Instruction {
    let ClaimArgs {
        proof,
        amount,
        bump,
    } = args;

    let ClaimAccounts {
        config,
        reward_distribution_account,
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
            reward_distribution_account,
            claimant,
            claim_status,
            payer,
            system_program,
        }
        .to_account_metas(None),
    }
}
