use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::RakuraiActivationConfigAccount;

/// Arguments for initializing the global config account.
pub struct InitializeArgs {
    pub authority: Pubkey,
    pub block_builder_authority: Pubkey,
    pub block_builder_commission_account: Pubkey,
    pub block_builder_commission_bps: u16,
    pub bump: u8,
}

/// Accounts required to initialize the config account.
pub struct InitializeAccounts {
    pub config: Pubkey,
    pub system_program: Pubkey,
    pub initializer: Pubkey,
}

/// Builds the `initialize` instruction for creating the config account.
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

/// Arguments to update the global config account.
pub struct UpdateConfigArgs {
    new_config: RakuraiActivationConfigAccount,
}

/// Accounts required to perform the config update.
pub struct UpdateConfigAccounts {
    pub config: Pubkey,
    pub authority: Pubkey,
}

/// Builds the `update_config` instruction for modifying config values.
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

/// Arguments for initializing a validator’s Rakurai Activation Account (RAA).
pub struct InitializeRakuraiActivationAccountArgs {
    pub validator_commission_bps: u16,
    pub bump: u8,
}

/// Required accounts to initialize a Rakurai Activation Account (RAA).
pub struct InitializeRakuraiActivationAccountAccounts {
    pub config: Pubkey,
    pub signer: Pubkey,
    pub system_program: Pubkey,
    pub activation_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub validator_identity_account: Pubkey,
}

/// Builds the `initialize_rakurai_activation_account` instruction.
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
        activation_account,
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
            activation_account,
            validator_vote_account,
            validator_identity_account,
        }
        .to_account_metas(None),
    }
}

/// Arguments for updating approval status of Rakurai Activation Account (RAA) account.
pub struct UpdateRakuraiActivationApprovalArgs {
    pub grant_approval: bool,
    pub hash: Option<[u8; 64]>,
}

/// Accounts required to approve/reject Rakurai Activation Account (RAA) activation.
pub struct UpdateRakuraiActivationApprovalAccounts {
    pub config: Pubkey,
    pub activation_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}

/// Builds the `update_rakurai_activation_approval` instruction.
pub fn update_rakurai_activation_approval_ix(
    program_id: Pubkey,
    args: UpdateRakuraiActivationApprovalArgs,
    accounts: UpdateRakuraiActivationApprovalAccounts,
) -> Instruction {
    let UpdateRakuraiActivationApprovalArgs {
        grant_approval,
        hash,
    } = args;

    let UpdateRakuraiActivationApprovalAccounts {
        config,
        activation_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRakuraiActivationApproval {
            grant_approval,
            hash,
        }
        .data(),
        accounts: crate::accounts::UpdateRakuraiActivationApproval {
            config,
            validator_identity_account,
            activation_account,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Arguments to update commission rate for validator/block builder.
pub struct UpdateRakuraiActivationCommissionArgs {
    pub commission_bps: u16,
}

/// Accounts required to perform commission update.
pub struct UpdateRakuraiActivationCommissionAccounts {
    pub config: Pubkey,
    pub activation_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}

/// Builds the `update_rakurai_activation_commission` instruction.
pub fn update_rakurai_activation_commission_ix(
    program_id: Pubkey,
    args: UpdateRakuraiActivationCommissionArgs,
    accounts: UpdateRakuraiActivationCommissionAccounts,
) -> Instruction {
    let UpdateRakuraiActivationCommissionArgs { commission_bps } = args;

    let UpdateRakuraiActivationCommissionAccounts {
        config,
        activation_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateRakuraiActivationCommission { commission_bps }.data(),
        accounts: crate::accounts::UpdateRakuraiActivationCommission {
            config,
            validator_identity_account,
            activation_account,
            signer,
        }
        .to_account_metas(None),
    }
}

/// Placeholder struct for closing Rakurai Activation Account (RAA) (no args).
pub struct CloseRakuraiActivationAccountArgs;

/// Accounts required to close and claim a Rakurai Activation Account (RAA) rent.
pub struct CloseRakuraiActivationAccounts {
    pub config: Pubkey,
    pub activation_account: Pubkey,
    pub validator_identity_account: Pubkey,
    pub signer: Pubkey,
}

/// Builds the `close_rakurai_activation_account` instruction.
pub fn close_rakurai_activation_account_ix(
    program_id: Pubkey,
    _args: CloseRakuraiActivationAccountArgs,
    accounts: CloseRakuraiActivationAccounts,
) -> Instruction {
    let CloseRakuraiActivationAccounts {
        config,
        activation_account,
        validator_identity_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRakuraiActivationAccount {}.data(),
        accounts: crate::accounts::CloseRakuraiActivationAccount {
            config,
            validator_identity_account,
            activation_account,
            signer,
        }
        .to_account_metas(None),
    }
}
