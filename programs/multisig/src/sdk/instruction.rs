use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::Config;

pub struct InitializeArgs {
    pub authority: Pubkey,
    block_builder_authority: Pubkey,
    block_builder_commission_account: Pubkey,
    block_builder_commission_bps: u16,
    max_validator_commission_bps: u16,
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
        block_builder_authority,
        block_builder_commission_account,
        block_builder_commission_bps,
        max_validator_commission_bps,
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
            block_builder_authority,
            block_builder_commission_account,
            block_builder_commission_bps,
            max_validator_commission_bps,
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

pub struct InitializeMultiSigAccountArgs {
    pub validator_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeMultiSigAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_vote_account: Pubkey,
}
pub fn initialize_multi_sig_account_ix(
    program_id: Pubkey,
    args: InitializeMultiSigAccountArgs,
    accounts: InitializeMultiSigAccountAccounts,
) -> Instruction {
    let InitializeMultiSigAccountArgs {
        validator_commission_bps,
        bump,
    } = args;

    let InitializeMultiSigAccountAccounts {
        config,
        multisig_account,
        system_program,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeMultiSigAccount {
            validator_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeMultiSigAccount {
            config,
            signer,
            system_program,
            multisig_account,
            validator_vote_account,
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

pub struct CloseMultiSigAccountArgs;
pub struct CloseMultiSigAccounts {
    pub config: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub signer: Pubkey,
}
pub fn close_multi_sig_account_ix(
    program_id: Pubkey,
    _args: CloseMultiSigAccountArgs,
    accounts: CloseMultiSigAccounts,
) -> Instruction {
    let CloseMultiSigAccounts {
        config,
        multisig_account,
        validator_vote_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseMultiSigAccount {}.data(),
        accounts: crate::accounts::CloseMultiSigAccount {
            config,
            validator_vote_account,
            multisig_account,
            signer,
        }
        .to_account_metas(None),
    }
}
