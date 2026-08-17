//! Instruction builders for off-chain clients.

use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::state::Config;

pub struct InitGlobalArgs {
    pub config: Config,
}

pub struct InitGlobalAccounts {
    pub manager: Pubkey,
    pub global: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_global_ix(
    program_id: Pubkey,
    args: InitGlobalArgs,
    accounts: InitGlobalAccounts,
) -> Instruction {
    let InitGlobalArgs { config } = args;
    let InitGlobalAccounts {
        manager,
        global,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitGlobal { config }.data(),
        accounts: crate::accounts::InitGlobal {
            manager,
            global,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateGlobalArgs {
    pub config: Config,
}

pub struct UpdateGlobalAccounts {
    pub manager: Pubkey,
    pub global: Pubkey,
    pub system_program: Pubkey,
}

pub fn update_global_ix(
    program_id: Pubkey,
    args: UpdateGlobalArgs,
    accounts: UpdateGlobalAccounts,
) -> Instruction {
    let UpdateGlobalArgs { config } = args;
    let UpdateGlobalAccounts {
        manager,
        global,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateGlobal { config }.data(),
        accounts: crate::accounts::UpdateGlobal {
            manager,
            global,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct InitValidatorArgs {
    pub operator: Pubkey,
}

pub struct InitValidatorAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_validator_ix(
    program_id: Pubkey,
    args: InitValidatorArgs,
    accounts: InitValidatorAccounts,
) -> Instruction {
    let InitValidatorArgs { operator } = args;
    let InitValidatorAccounts {
        manager,
        vote,
        global,
        validator,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitValidator { operator }.data(),
        accounts: crate::accounts::InitValidator {
            manager,
            vote,
            global,
            validator,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateValidatorArgs {
    pub config: Config,
}

pub struct UpdateValidatorAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
    pub system_program: Pubkey,
}

pub fn update_validator_ix(
    program_id: Pubkey,
    args: UpdateValidatorArgs,
    accounts: UpdateValidatorAccounts,
) -> Instruction {
    let UpdateValidatorArgs { config } = args;
    let UpdateValidatorAccounts {
        manager,
        vote,
        global,
        validator,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateValidator { config }.data(),
        accounts: crate::accounts::UpdateValidator {
            manager,
            vote,
            global,
            validator,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct SetOperatorArgs {
    pub operator: Pubkey,
}

pub struct SetOperatorAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
}

pub fn set_operator_ix(
    program_id: Pubkey,
    args: SetOperatorArgs,
    accounts: SetOperatorAccounts,
) -> Instruction {
    let SetOperatorArgs { operator } = args;
    let SetOperatorAccounts {
        manager,
        vote,
        global,
        validator,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::SetOperator { operator }.data(),
        accounts: crate::accounts::SetOperator {
            manager,
            vote,
            global,
            validator,
        }
        .to_account_metas(None),
    }
}

pub struct CloseGlobalAccounts {
    pub manager: Pubkey,
    pub global: Pubkey,
}

pub fn close_global_ix(program_id: Pubkey, accounts: CloseGlobalAccounts) -> Instruction {
    let CloseGlobalAccounts { manager, global } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseGlobal {}.data(),
        accounts: crate::accounts::CloseGlobal { manager, global }.to_account_metas(None),
    }
}

pub struct CloseValidatorAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
}

pub fn close_validator_ix(program_id: Pubkey, accounts: CloseValidatorAccounts) -> Instruction {
    let CloseValidatorAccounts {
        manager,
        vote,
        global,
        validator,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::CloseValidator {}.data(),
        accounts: crate::accounts::CloseValidator {
            manager,
            vote,
            global,
            validator,
        }
        .to_account_metas(None),
    }
}

pub struct InitProposalArgs {
    pub config: Config,
}

pub struct InitProposalAccounts {
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_proposal_ix(
    program_id: Pubkey,
    args: InitProposalArgs,
    accounts: InitProposalAccounts,
) -> Instruction {
    let InitProposalArgs { config } = args;
    let InitProposalAccounts {
        operator,
        vote,
        validator,
        proposal,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitProposal { config }.data(),
        accounts: crate::accounts::InitProposal {
            operator,
            vote,
            validator,
            proposal,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateProposalArgs {
    pub config: Config,
}

pub struct UpdateProposalAccounts {
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub system_program: Pubkey,
}

pub fn update_proposal_ix(
    program_id: Pubkey,
    args: UpdateProposalArgs,
    accounts: UpdateProposalAccounts,
) -> Instruction {
    let UpdateProposalArgs { config } = args;
    let UpdateProposalAccounts {
        operator,
        vote,
        validator,
        proposal,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateProposal { config }.data(),
        accounts: crate::accounts::UpdateProposal {
            operator,
            vote,
            validator,
            proposal,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct ApproveProposalAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub operator: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub system_program: Pubkey,
}

pub fn approve_proposal_ix(program_id: Pubkey, accounts: ApproveProposalAccounts) -> Instruction {
    let ApproveProposalAccounts {
        manager,
        vote,
        operator,
        global,
        validator,
        proposal,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::ApproveProposal {}.data(),
        accounts: crate::accounts::ApproveProposal {
            manager,
            vote,
            operator,
            global,
            validator,
            proposal,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct RejectProposalAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub operator: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
}

pub fn reject_proposal_ix(program_id: Pubkey, accounts: RejectProposalAccounts) -> Instruction {
    let RejectProposalAccounts {
        manager,
        vote,
        operator,
        global,
        validator,
        proposal,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::RejectProposal {}.data(),
        accounts: crate::accounts::RejectProposal {
            manager,
            vote,
            operator,
            global,
            validator,
            proposal,
        }
        .to_account_metas(None),
    }
}
