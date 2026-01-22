//! This module contains functions that build instructions to interact with the block-reward-distribution program.
use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::TipManagerBumps;

pub struct InitializeTipManagerArgs {
    pub _bumps: TipManagerBumps,
}

pub struct InitializeTipManagerAccounts {
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
pub fn initialize_tip_manager_ix(
    program_id: Pubkey,
    args: InitializeTipManagerArgs,
    accounts: InitializeTipManagerAccounts,
) -> Instruction {
    let InitializeTipManagerArgs { _bumps } = args;

    let InitializeTipManagerAccounts {
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
        data: crate::instruction::InitializeTipManager { _bumps }.data(),
        accounts: crate::accounts::InitializeTipManager {
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

pub struct CloseTipManagerArgs;

pub struct CloseTipManagerAccounts {
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
pub fn close_tip_manager_ix(
    program_id: Pubkey,
    _args: CloseTipManagerArgs,
    accounts: CloseTipManagerAccounts,
) -> Instruction {
    let CloseTipManagerAccounts {
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
        data: crate::instruction::CloseTipManager {}.data(),
        accounts: crate::accounts::CloseTipManager {
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

pub struct ClaimTipsArgs;

pub struct ClaimTipsAccounts {
    pub tip_manager_config: Pubkey,
    pub rakurai_tip_account_0: Pubkey,
    pub rakurai_tip_account_1: Pubkey,
    pub rakurai_tip_account_2: Pubkey,
    pub rakurai_tip_account_3: Pubkey,
    pub rakurai_tip_account_4: Pubkey,
    pub rakurai_tip_account_5: Pubkey,
    pub rakurai_tip_account_6: Pubkey,
    pub rakurai_tip_account_7: Pubkey,
    pub validator_tip_receiver_account: Pubkey,
    pub block_builder_commission_account: Pubkey,
    pub signer: Pubkey,
}

/// Builds the instruction to initialize tip manager program.
pub fn claim_tips_ix(
    program_id: Pubkey,
    _args: ClaimTipsArgs,
    accounts: ClaimTipsAccounts,
) -> Instruction {
    let ClaimTipsAccounts {
        tip_manager_config,
        rakurai_tip_account_0,
        rakurai_tip_account_1,
        rakurai_tip_account_2,
        rakurai_tip_account_3,
        rakurai_tip_account_4,
        rakurai_tip_account_5,
        rakurai_tip_account_6,
        rakurai_tip_account_7,
        validator_tip_receiver_account,
        block_builder_commission_account,
        signer,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ClaimTips {}.data(),
        accounts: crate::accounts::ClaimTips {
            tip_manager_config,
            rakurai_tip_account_0,
            rakurai_tip_account_1,
            rakurai_tip_account_2,
            rakurai_tip_account_3,
            rakurai_tip_account_4,
            rakurai_tip_account_5,
            rakurai_tip_account_6,
            rakurai_tip_account_7,
            validator_tip_receiver_account,
            block_builder_commission_account,
            signer,
        }
        .to_account_metas(None),
    }
}

pub struct ChangeTipReceiverArgs;

pub struct ChangeTipReceiverAccounts {
    pub tip_manager_config: Pubkey,
    pub old_tip_receiver: Pubkey,
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

/// Builds the instruction to initialize tip manager program.
pub fn change_tip_receiver_ix(
    program_id: Pubkey,
    _args: ChangeTipReceiverArgs,
    accounts: ChangeTipReceiverAccounts,
) -> Instruction {
    let ChangeTipReceiverAccounts {
        tip_manager_config,
        old_tip_receiver,
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
            old_tip_receiver,
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
