//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
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

pub struct ChangeTipReceiverAccounts {
    pub tip_manager_config: Pubkey,
    pub rakurai_activation_account: Pubkey,
    pub validator_vote_account: Pubkey,
    pub old_tip_receiver: Pubkey,
    pub reward_distribution_program: Pubkey,
    pub new_tip_receiver: Pubkey,
    pub block_builder_commission_account: Pubkey,
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

/// Builds the instruction to drain pending tips and rotate the tip receiver.
pub fn change_tip_receiver_ix(
    program_id: Pubkey,
    _args: ChangeTipReceiverArgs,
    accounts: ChangeTipReceiverAccounts,
) -> Instruction {
    let ChangeTipReceiverAccounts {
        tip_manager_config,
        rakurai_activation_account,
        validator_vote_account,
        old_tip_receiver,
        reward_distribution_program,
        new_tip_receiver,
        block_builder_commission_account,
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
            rakurai_activation_account,
            validator_vote_account,
            old_tip_receiver,
            reward_distribution_program,
            new_tip_receiver,
            block_builder_commission_account,
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

pub struct ChangeBlockBuilderArgs {
    pub block_builder_commission_bps: u64,
}

pub struct ChangeBlockBuilderAccounts {
    pub tip_manager_config: Pubkey,
    pub validator_tip_receiver_account: Pubkey,
    pub old_block_builder: Pubkey,
    pub new_block_builder: Pubkey,
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
pub fn change_block_builder_ix(
    program_id: Pubkey,
    args: ChangeBlockBuilderArgs,
    accounts: ChangeBlockBuilderAccounts,
) -> Instruction {
    let ChangeBlockBuilderArgs {
        block_builder_commission_bps,
    } = args;

    let ChangeBlockBuilderAccounts {
        tip_manager_config,
        validator_tip_receiver_account,
        old_block_builder,
        new_block_builder,
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
        data: crate::instruction::ChangeBlockBuilder {
            block_builder_commission_bps,
        }
        .data(),
        accounts: crate::accounts::ChangeBlockBuilder {
            tip_manager_config,
            validator_tip_receiver_account,
            old_block_builder,
            new_block_builder,
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
