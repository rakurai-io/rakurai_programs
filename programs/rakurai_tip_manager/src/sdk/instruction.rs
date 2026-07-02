//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::AccountMeta,
    solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::RakuraiTipManagerBumps;

pub struct InitializeRakuraiTipManagerArgs {
    pub bumps: RakuraiTipManagerBumps,
}

pub struct InitializeRakuraiTipManagerAccounts {
    pub tip_manager_config: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub system_program: Pubkey,
    pub payer: Pubkey,
}

/// Builds the instruction to initialize tip manager program accounts.
pub fn initialize_rakurai_tip_manager_ix(
    program_id: Pubkey,
    args: InitializeRakuraiTipManagerArgs,
    accounts: InitializeRakuraiTipManagerAccounts,
) -> Instruction {
    let InitializeRakuraiTipManagerArgs { bumps } = args;

    let InitializeRakuraiTipManagerAccounts {
        tip_manager_config,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        system_program,
        payer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitializeRakuraiTipManager { bumps }.data(),
        accounts: crate::accounts::InitializeRakuraiTipManager {
            tip_manager_config,
            rakurai_tip_account_0,
            rakurai_tip_account_1,
            rakurai_tip_account_2,
            rakurai_tip_account_3,
            rakurai_tip_account_4,
            rakurai_tip_account_5,
            rakurai_tip_account_6,
            rakurai_tip_account_7,
            system_program,
            payer,
        }
        .to_account_metas(None),
    }
}

pub struct CloseRakuraiTipManagerArgs;

pub struct CloseRakuraiTipManagerAccounts {
    pub tip_manager_config: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub system_program: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to close tip manager program accounts.
pub fn close_rakurai_tip_manager_ix(
    program_id: Pubkey,
    _args: CloseRakuraiTipManagerArgs,
    accounts: CloseRakuraiTipManagerAccounts,
) -> Instruction {
    let CloseRakuraiTipManagerAccounts {
        tip_manager_config,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        system_program,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseRakuraiTipManager {}.data(),
        accounts: crate::accounts::CloseRakuraiTipManager {
            tip_manager_config,
            rakurai_tip_account_0,
            rakurai_tip_account_1,
            rakurai_tip_account_2,
            rakurai_tip_account_3,
            rakurai_tip_account_4,
            rakurai_tip_account_5,
            rakurai_tip_account_6,
            rakurai_tip_account_7,
            system_program,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct ChangeTipReceiverArgs;

/// Legacy account list (unchanged for existing clients).
pub struct ChangeTipReceiverAccounts {
    pub tip_manager_config: Pubkey,
    pub old_tip_receiver: Pubkey,
    pub new_tip_receiver: Pubkey,
    pub client_commission_account: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub signer: Pubkey,
}

/// Builds the legacy instruction to drain pending tips and rotate the tip receiver.
pub fn change_tip_receiver_ix(
    program_id: Pubkey,
    _args: ChangeTipReceiverArgs,
    accounts: ChangeTipReceiverAccounts,
) -> Instruction {
    let ChangeTipReceiverAccounts {
        tip_manager_config,
        old_tip_receiver,
        new_tip_receiver,
        client_commission_account,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ChangeTipReceiver {}.data(),
        accounts: crate::accounts::ChangeTipReceiver {
            tip_manager_config,
            old_tip_receiver,
            new_tip_receiver,
            client_commission_account,
            rakurai_tip_account_0,
            rakurai_tip_account_1,
            rakurai_tip_account_2,
            rakurai_tip_account_3,
            rakurai_tip_account_4,
            rakurai_tip_account_5,
            rakurai_tip_account_6,
            rakurai_tip_account_7,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct ChangeTipReceiverV1Args;

pub struct ChangeTipReceiverV1Accounts {
    pub tip_manager_config: Pubkey,
    pub old_tip_receiver: Pubkey,
    pub new_tip_receiver: Pubkey,
    pub client_commission_account: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub signer: Pubkey,
    /// Appended as `remaining_accounts[0]` (enabled RAA PDA for signer).
    pub rakurai_activation_account: Pubkey,
    /// Appended as `remaining_accounts[1]` (reward distribution program id).
    pub reward_distribution_program: Pubkey,
}

/// Drains pending tips and rotates config to the TCA PDA (RAA + vote + TCA validation).
pub fn change_tip_receiver_v1_ix(
    program_id: Pubkey,
    _args: ChangeTipReceiverV1Args,
    accounts: ChangeTipReceiverV1Accounts,
) -> Instruction {
    let ChangeTipReceiverV1Accounts {
        tip_manager_config,
        old_tip_receiver,
        new_tip_receiver,
        client_commission_account,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        signer,
        rakurai_activation_account,
        reward_distribution_program,
    } = accounts;

    let mut account_metas = crate::accounts::ChangeTipReceiverV1 {
        tip_manager_config,
        old_tip_receiver,
        new_tip_receiver,
        client_commission_account,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        signer,
    }
    .to_account_metas(None);
    account_metas.push(AccountMeta::new_readonly(rakurai_activation_account, false));
    account_metas.push(AccountMeta::new_readonly(
        reward_distribution_program,
        false,
    ));
    Instruction {
        program_id,
        data: crate::instruction::ChangeTipReceiverV1 {}.data(),
        accounts: account_metas,
    }
}

pub struct ChangeClientArgs {
    pub client_commission_bps: u64,
}

pub struct ChangeClientAccounts {
    pub tip_manager_config: Pubkey,
    pub validator_tip_receiver_account: Pubkey,
    pub old_client: Pubkey,
    pub new_client: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to initialize tip manager program.
pub fn change_client_ix(
    program_id: Pubkey,
    args: ChangeClientArgs,
    accounts: ChangeClientAccounts,
) -> Instruction {
    let ChangeClientArgs {
        client_commission_bps,
    } = args;

    let ChangeClientAccounts {
        tip_manager_config,
        validator_tip_receiver_account,
        old_client,
        new_client,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ChangeClient {
            client_commission_bps,
        }
        .data(),
        accounts: crate::accounts::ChangeClient {
            tip_manager_config,
            validator_tip_receiver_account,
            old_client,
            new_client,
            rakurai_tip_account_0,
            rakurai_tip_account_1,
            rakurai_tip_account_2,
            rakurai_tip_account_3,
            rakurai_tip_account_4,
            rakurai_tip_account_5,
            rakurai_tip_account_6,
            rakurai_tip_account_7,
            signer,
        }
        .to_account_metas(None),
    }
}
