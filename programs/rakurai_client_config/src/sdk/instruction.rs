//! Instruction builders for off-chain clients.

use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::state::{Config, ConfigLimits};

pub struct InitGlobalArgs {
    pub config: Config,
    pub limits: ConfigLimits,
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
    let InitGlobalArgs { config, limits } = args;
    let InitGlobalAccounts {
        manager,
        global,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::InitGlobal { config, limits }.data(),
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

pub fn migrate_global_to_v2_ix(program_id: Pubkey, accounts: UpdateGlobalAccounts) -> Instruction {
    let UpdateGlobalAccounts {
        manager,
        global,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::MigrateGlobalToV2 {}.data(),
        accounts: crate::accounts::MigrateGlobalToV2 {
            manager,
            global,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateGlobalLimitsArgs {
    pub limits: ConfigLimits,
}

pub struct UpdateGlobalLimitsAccounts {
    pub manager: Pubkey,
    pub global: Pubkey,
}

pub fn update_global_limits_ix(
    program_id: Pubkey,
    args: UpdateGlobalLimitsArgs,
    accounts: UpdateGlobalLimitsAccounts,
) -> Instruction {
    let UpdateGlobalLimitsArgs { limits } = args;
    let UpdateGlobalLimitsAccounts { manager, global } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateGlobalLimits { limits }.data(),
        accounts: crate::accounts::UpdateGlobalLimits { manager, global }.to_account_metas(None),
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

pub fn migrate_validator_to_v2_ix(
    program_id: Pubkey,
    accounts: UpdateValidatorAccounts,
) -> Instruction {
    let UpdateValidatorAccounts {
        manager,
        vote,
        global,
        validator,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::MigrateValidatorToV2 {}.data(),
        accounts: crate::accounts::MigrateValidatorToV2 {
            manager,
            vote,
            global,
            validator,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub struct UpdateValidatorLimitsArgs {
    pub limits: ConfigLimits,
}

pub struct UpdateValidatorLimitsAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
}

pub fn update_validator_limits_ix(
    program_id: Pubkey,
    args: UpdateValidatorLimitsArgs,
    accounts: UpdateValidatorLimitsAccounts,
) -> Instruction {
    let UpdateValidatorLimitsArgs { limits } = args;
    let UpdateValidatorLimitsAccounts {
        manager,
        vote,
        global,
        validator,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::UpdateValidatorLimits { limits }.data(),
        accounts: crate::accounts::UpdateValidatorLimits {
            manager,
            vote,
            global,
            validator,
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

pub fn migrate_proposal_to_v2_ix(
    program_id: Pubkey,
    accounts: UpdateProposalAccounts,
) -> Instruction {
    let UpdateProposalAccounts {
        operator,
        vote,
        validator,
        proposal,
        system_program,
    } = accounts;

    Instruction {
        program_id,
        data: crate::instruction::MigrateProposalToV2 {}.data(),
        accounts: crate::accounts::MigrateProposalToV2 {
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

pub struct StagingLenArgs {
    pub expected_len: u32,
}

pub struct StagingChunkArgs {
    pub data: Vec<u8>,
}

#[derive(Clone, Copy)]
pub struct GlobalStagingAccounts {
    pub manager: Pubkey,
    pub global: Pubkey,
    pub staging: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_global_staging_ix(
    program_id: Pubkey,
    args: StagingLenArgs,
    accounts: GlobalStagingAccounts,
) -> Instruction {
    let StagingLenArgs { expected_len } = args;
    let GlobalStagingAccounts {
        manager,
        global,
        staging,
        system_program,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::InitGlobalStaging { expected_len }.data(),
        accounts: crate::accounts::InitGlobalStaging {
            manager,
            global,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn write_global_staging_ix(
    program_id: Pubkey,
    args: StagingChunkArgs,
    accounts: GlobalStagingAccounts,
) -> Instruction {
    let StagingChunkArgs { data } = args;
    let GlobalStagingAccounts {
        manager,
        staging,
        system_program,
        ..
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::WriteGlobalStaging { data }.data(),
        accounts: crate::accounts::WriteGlobalStaging {
            manager,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn commit_global_staging_ix(
    program_id: Pubkey,
    accounts: GlobalStagingAccounts,
) -> Instruction {
    let GlobalStagingAccounts {
        manager,
        global,
        staging,
        system_program,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::CommitGlobalStaging {}.data(),
        accounts: crate::accounts::CommitGlobalStaging {
            manager,
            global,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

#[derive(Clone, Copy)]
pub struct AbortGlobalStagingAccounts {
    pub manager: Pubkey,
    pub staging: Pubkey,
}

pub fn abort_global_staging_ix(
    program_id: Pubkey,
    accounts: AbortGlobalStagingAccounts,
) -> Instruction {
    let AbortGlobalStagingAccounts { manager, staging } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::AbortGlobalStaging {}.data(),
        accounts: crate::accounts::AbortGlobalStaging { manager, staging }.to_account_metas(None),
    }
}

#[derive(Clone, Copy)]
pub struct ValidatorStagingAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub global: Pubkey,
    pub validator: Pubkey,
    pub staging: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_validator_staging_ix(
    program_id: Pubkey,
    args: StagingLenArgs,
    accounts: ValidatorStagingAccounts,
) -> Instruction {
    let StagingLenArgs { expected_len } = args;
    let ValidatorStagingAccounts {
        manager,
        vote,
        global,
        validator,
        staging,
        system_program,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::InitValidatorStaging { expected_len }.data(),
        accounts: crate::accounts::InitValidatorStaging {
            manager,
            vote,
            global,
            validator,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn write_validator_staging_ix(
    program_id: Pubkey,
    args: StagingChunkArgs,
    accounts: ValidatorStagingAccounts,
) -> Instruction {
    let StagingChunkArgs { data } = args;
    let ValidatorStagingAccounts {
        manager,
        vote,
        staging,
        system_program,
        ..
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::WriteValidatorStaging { data }.data(),
        accounts: crate::accounts::WriteValidatorStaging {
            manager,
            vote,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn commit_validator_staging_ix(
    program_id: Pubkey,
    accounts: ValidatorStagingAccounts,
) -> Instruction {
    let ValidatorStagingAccounts {
        manager,
        vote,
        global,
        validator,
        staging,
        system_program,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::CommitValidatorStaging {}.data(),
        accounts: crate::accounts::CommitValidatorStaging {
            manager,
            vote,
            global,
            validator,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

#[derive(Clone, Copy)]
pub struct AbortValidatorStagingAccounts {
    pub manager: Pubkey,
    pub vote: Pubkey,
    pub staging: Pubkey,
}

pub fn abort_validator_staging_ix(
    program_id: Pubkey,
    accounts: AbortValidatorStagingAccounts,
) -> Instruction {
    let AbortValidatorStagingAccounts {
        manager,
        vote,
        staging,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::AbortValidatorStaging {}.data(),
        accounts: crate::accounts::AbortValidatorStaging {
            manager,
            vote,
            staging,
        }
        .to_account_metas(None),
    }
}

#[derive(Clone, Copy)]
pub struct ProposalStagingAccounts {
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub validator: Pubkey,
    pub proposal: Pubkey,
    pub staging: Pubkey,
    pub system_program: Pubkey,
}

pub fn init_proposal_staging_ix(
    program_id: Pubkey,
    args: StagingLenArgs,
    accounts: ProposalStagingAccounts,
) -> Instruction {
    let StagingLenArgs { expected_len } = args;
    let ProposalStagingAccounts {
        operator,
        vote,
        validator,
        staging,
        system_program,
        ..
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::InitProposalStaging { expected_len }.data(),
        accounts: crate::accounts::InitProposalStaging {
            operator,
            vote,
            validator,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn write_proposal_staging_ix(
    program_id: Pubkey,
    args: StagingChunkArgs,
    accounts: ProposalStagingAccounts,
) -> Instruction {
    let StagingChunkArgs { data } = args;
    let ProposalStagingAccounts {
        operator,
        vote,
        staging,
        system_program,
        ..
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::WriteProposalStaging { data }.data(),
        accounts: crate::accounts::WriteProposalStaging {
            operator,
            vote,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

pub fn commit_proposal_staging_ix(
    program_id: Pubkey,
    accounts: ProposalStagingAccounts,
) -> Instruction {
    let ProposalStagingAccounts {
        operator,
        vote,
        validator,
        proposal,
        staging,
        system_program,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::CommitProposalStaging {}.data(),
        accounts: crate::accounts::CommitProposalStaging {
            operator,
            vote,
            validator,
            proposal,
            staging,
            system_program,
        }
        .to_account_metas(None),
    }
}

#[derive(Clone, Copy)]
pub struct AbortProposalStagingAccounts {
    pub operator: Pubkey,
    pub vote: Pubkey,
    pub staging: Pubkey,
}

pub fn abort_proposal_staging_ix(
    program_id: Pubkey,
    accounts: AbortProposalStagingAccounts,
) -> Instruction {
    let AbortProposalStagingAccounts {
        operator,
        vote,
        staging,
    } = accounts;
    Instruction {
        program_id,
        data: crate::instruction::AbortProposalStaging {}.data(),
        accounts: crate::accounts::AbortProposalStaging {
            operator,
            vote,
            staging,
        }
        .to_account_metas(None),
    }
}
