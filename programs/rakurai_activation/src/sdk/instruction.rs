use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::RakuraiActivationConfigAccount;

pub struct InitializeArgs {
    pub authority: Pubkey,
    pub block_builder_authority: Pubkey,
    pub block_builder_commission_account: Pubkey,
    pub block_builder_commission_bps: u16,
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

pub struct UpdateConfigArgs {
    new_config: RakuraiActivationConfigAccount,
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

pub struct InitializeRakuraiActivationAccountArgs {
    pub validator_commission_bps: u16,
    pub bump: u8,
}
pub struct InitializeRakuraiActivationAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub validator_identity_account: Pubkey,
}
pub fn initialize_rakurai_activation_account_ix(
    program_id: Pubkey,
    args: InitializeRakuraiActivationAccountArgs,
    accounts: InitializeRakuraiActivationAccountAccounts,
) -> Instruction {
    let InitializeRakuraiActivationAccountArgs {
        validator_commission_bps,
        bump,
    } = args;

    let InitializeRakuraiActivationAccountAccounts {
        config,
        multisig_account,
        system_program,
        validator_vote_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRakuraiActivationAccount {
            validator_commission_bps,
            bump,
        }
        .data(),
        accounts: crate::accounts::InitializeRakuraiActivationAccount {
            config,
            signer,
            system_program,
            multisig_account,
            validator_vote_account,
            validator_identity_account,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateRakuraiActivationApprovalArgs {
    pub grant_approval: bool,
}
pub struct UpdateRakuraiActivationApprovalAccounts {
    pub config: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}
pub fn update_rakurai_activation_approval_ix(
    program_id: Pubkey,
    args: UpdateRakuraiActivationApprovalArgs,
    accounts: UpdateRakuraiActivationApprovalAccounts,
) -> Instruction {
    let UpdateRakuraiActivationApprovalArgs { grant_approval } = args;

    let UpdateRakuraiActivationApprovalAccounts {
        config,
        multisig_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRakuraiActivationApproval { grant_approval }.data(),
        accounts: crate::accounts::UpdateRakuraiActivationApproval {
            config,
            validator_identity_account,
            multisig_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateRakuraiActivationCommissionArgs {
    pub validator_commission_bps: Option<u16>,
}
pub struct UpdateRakuraiActivationCommissionAccounts {
    pub config: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}
pub fn update_rakurai_activation_commission_ix(
    program_id: Pubkey,
    args: UpdateRakuraiActivationCommissionArgs,
    accounts: UpdateRakuraiActivationCommissionAccounts,
) -> Instruction {
    let UpdateRakuraiActivationCommissionArgs {
        validator_commission_bps,
    } = args;

    let UpdateRakuraiActivationCommissionAccounts {
        config,
        multisig_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRakuraiActivationCommission {
            validator_commission_bps,
        }
        .data(),
        accounts: crate::accounts::UpdateRakuraiActivationCommission {
            config,
            validator_identity_account,
            multisig_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct CloseRakuraiActivationAccountArgs;
pub struct CloseRakuraiActivationAccounts {
    pub config: Pubkey,
    pub multisig_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}
pub fn close_rakurai_activation_account_ix(
    program_id: Pubkey,
    _args: CloseRakuraiActivationAccountArgs,
    accounts: CloseRakuraiActivationAccounts,
) -> Instruction {
    let CloseRakuraiActivationAccounts {
        config,
        multisig_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRakuraiActivationAccount {}.data(),
        accounts: crate::accounts::CloseRakuraiActivationAccount {
            config,
            validator_identity_account,
            multisig_account,
            signer,
        }
        .to_account_metas(None),
    }
}
