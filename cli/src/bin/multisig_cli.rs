use {
    clap::Parser,
    colored::*,
    multisig::sdk::{
        derive_config_account_address, derive_multisig_account_address,
        instruction::{
            close_multi_sig_account_ix, initialize_ix, initialize_multi_sig_account_ix,
            update_multi_sig_approval_ix, update_multi_sig_commission_ix, CloseMultiSigAccountArgs,
            CloseMultiSigAccounts, InitializeAccounts, InitializeArgs,
            InitializeMultiSigAccountAccounts, InitializeMultiSigAccountArgs,
            UpdateMultiSigApprovalAccounts, UpdateMultiSigApprovalArgs,
            UpdateMultiSigCommissionAccounts, UpdateMultiSigCommissionArgs,
        },
    },
    rakurai_cli::{
        clap_args::{
            Cli, ClosePdaArgs, Commands, InitConfigArgs, InitPdaArgs, SchedulerControlArgs,
            ShowPdaArgs, UpdateCommissionArgs,
        },
        display_multisig_account, get_multisig_account, get_vote_account,
        sign_and_send_transaction,
    },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        signature::{Keypair, Signer},
        system_program,
    },
    std::sync::Arc,
};

fn process_init_config(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: InitConfigArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();

    let config_authority = args.config_authority.unwrap_or(signer_pubkey);
    let block_builder_authority = args.block_builder_authority.unwrap_or(signer_pubkey);
    let block_builder_commission_bps = args.block_builder_commission_bps.unwrap_or(1000);
    let block_builder_commission_account = args
        .block_builder_commission_account
        .unwrap_or(signer_pubkey);

    let (config_pda, bump) = derive_config_account_address(&multisig::id());
    println!("📌 Derived Config PDA: {} (Bump: {})", config_pda, bump);

    println!(
        "{} {}\n{} {}\n{} {}\n{} {}",
        "🚀 Block builder commission:".green(),
        block_builder_commission_bps,
        "🏦 Commission Account:".blue(),
        block_builder_commission_account,
        "🔑 Authority:".purple(),
        block_builder_authority,
        "🔗 Signer and Config Authority:".cyan(),
        signer_pubkey
    );

    let initialize_instruction = initialize_ix(
        multisig::id(),
        InitializeArgs {
            authority: config_authority,
            block_builder_commission_bps,
            block_builder_commission_account,
            block_builder_authority,
            bump,
        },
        InitializeAccounts {
            config: config_pda,
            system_program: system_program::id(),
            initializer: signer_pubkey,
        },
    );

    sign_and_send_transaction(rpc_client, initialize_instruction, &kp)
}

fn process_init_pda(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: InitPdaArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let validator_commission_bps = args.commission_bps;
    let vote_pubkey = args.vote_pubkey;

    let vote_state = get_vote_account(rpc_client.clone(), vote_pubkey)?;
    if vote_state.node_pubkey != signer_pubkey {
        eprintln!(
            "❌ Unauthorized signer! Expected: {:?}, Found: {:?}",
            vote_state.node_pubkey, signer_pubkey
        );
        return Err("Unauthorized signer".into());
    }

    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) =
        derive_multisig_account_address(&multisig::id(), &vote_state.node_pubkey);

    println!("📌 Derived Multisig PDA: {} (Bump: {})", multisig_pda, bump);
    println!(
        "{} {}\n{} {}\n{} {}",
        "🚀 Validator commission:".green(),
        validator_commission_bps,
        "🏦 Vote Pubkey:".blue(),
        vote_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );

    let initialize_instruction = initialize_multi_sig_account_ix(
        multisig::id(),
        InitializeMultiSigAccountArgs {
            validator_commission_bps,
            bump,
        },
        InitializeMultiSigAccountAccounts {
            config: config_pda,
            system_program: system_program::id(),
            validator_vote_account: vote_pubkey,
            validator_identity_account: vote_state.node_pubkey,
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );

    sign_and_send_transaction(rpc_client.clone(), initialize_instruction, &kp)
}

fn process_scheduler_control(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: SchedulerControlArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();

    let disable_scheduler = args.disable_scheduler;
    let identity_pubkey = args.identity_pubkey;

    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) = derive_multisig_account_address(&multisig::id(), &identity_pubkey);
    let multisig_account = get_multisig_account(rpc_client.clone(), multisig_pda)?;
    if !(identity_pubkey == signer_pubkey
        || multisig_account.block_builder_authority == signer_pubkey)
    {
        eprintln!(
            "❌ Unauthorized Signer! Expected: Validator({}) or BlockBuilder({}), Found: {}",
            identity_pubkey, multisig_account.block_builder_authority, signer_pubkey
        );
        return Err("Unauthorized signer".into());
    }

    println!("📌 Derived Multisig PDA: {} (Bump: {})", multisig_pda, bump);
    println!(
        "{} {}\n{} {}\n{} {}",
        "🚀 Scheduler Enabled:".green(),
        disable_scheduler,
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );
    let update_approval_instruction = update_multi_sig_approval_ix(
        multisig::id(),
        UpdateMultiSigApprovalArgs {
            grant_approval: disable_scheduler,
        },
        UpdateMultiSigApprovalAccounts {
            config: config_pda,
            validator_identity_account: identity_pubkey,
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_approval_instruction, &kp)
}

fn process_update_commission(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: UpdateCommissionArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let commission_bps = args.commission_bps;
    let identity_pubkey = args.identity_pubkey;

    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) = derive_multisig_account_address(&multisig::id(), &identity_pubkey);

    println!("📌 Derived Multisig PDA: {} (Bump: {})", multisig_pda, bump);
    println!(
        "{} {}\n{} {}\n{} {}",
        "🚀 Commission BPS:".green(),
        commission_bps.unwrap_or_default(),
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );
    let multisig_account = get_multisig_account(rpc_client.clone(), multisig_pda)?;
    if !(signer_pubkey == identity_pubkey
        || signer_pubkey == multisig_account.block_builder_authority)
    {
        eprintln!(
            "❌ Unauthorized Signer! Expected: Validator({}) or BlockBuilder({}), Found: {}",
            identity_pubkey, multisig_account.block_builder_authority, signer_pubkey
        );
        return Err("Unauthorized signer".into());
    }
    if signer_pubkey == identity_pubkey {
        if let Some(new_commission) = commission_bps {
            if new_commission == multisig_account.validator_commission_bps {
                eprintln!("❌ No transaction required, commission value is unchanged.");
                return Err("No update needed".into());
            }
        } else {
            eprintln!("❌ No commission value provided for validator update.");
            return Err("Missing commission value".into());
        }
    } else if signer_pubkey == multisig_account.block_builder_authority {
        if commission_bps.is_some() {
            eprintln!("❌ Block Builder is not allowed to update commission.");
            return Err("Unauthorized update".into());
        }
    }

    let update_commission_instruction = update_multi_sig_commission_ix(
        multisig::id(),
        UpdateMultiSigCommissionArgs {
            validator_commission_bps: commission_bps,
        },
        UpdateMultiSigCommissionAccounts {
            config: config_pda,
            validator_identity_account: identity_pubkey,
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_commission_instruction, &kp)
}

fn process_close(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: ClosePdaArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let signer_pubkey = kp.pubkey();
    let identity_pubkey = args.identity_pubkey;

    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) = derive_multisig_account_address(&multisig::id(), &identity_pubkey);

    let multisig_account = get_multisig_account(rpc_client.clone(), multisig_pda)?;
    if multisig_account.block_builder_authority != signer_pubkey {
        eprintln!(
            "❌ Unauthorized Signer! Expected: BlockBuilder({}), Found: {}",
            multisig_account.block_builder_authority, signer_pubkey
        );
        return Err("Unauthorized signer".into());
    }

    println!("📌 Derived Multisig PDA: {} (Bump: {})", multisig_pda, bump);
    println!(
        "{} {}\n{} {}",
        "🏦 Identity Pubkey:".blue(),
        identity_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );
    let update_approval_instruction = close_multi_sig_account_ix(
        multisig::id(),
        CloseMultiSigAccountArgs {},
        CloseMultiSigAccounts {
            config: config_pda,
            validator_identity_account: identity_pubkey,
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );
    sign_and_send_transaction(rpc_client.clone(), update_approval_instruction, &kp)
}

fn process_show(
    rpc_client: Arc<RpcClient>,
    args: ShowPdaArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity_pubkey = args.identity_pubkey;

    let (multisig_pda, _) = derive_multisig_account_address(&multisig::id(), &identity_pubkey);

    let multisig_account = get_multisig_account(rpc_client.clone(), multisig_pda)?;
    println!("📌 PDA: {}", multisig_pda);
    display_multisig_account(multisig_account);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.rpc.clone(),
        CommitmentConfig::confirmed(),
    ));

    match &cli.command {
        Commands::InitPda(args) => process_init_pda(rpc_client.clone(), cli.keypair, args.clone())?,
        Commands::InitConfig(args) => {
            process_init_config(rpc_client.clone(), cli.keypair, args.clone())?
        }
        Commands::SchedulerControl(args) => {
            process_scheduler_control(rpc_client.clone(), cli.keypair, args.clone())?
        }
        Commands::UpdateCommission(args) => {
            process_update_commission(rpc_client.clone(), cli.keypair, args.clone())?
        }
        Commands::Close(args) => process_close(rpc_client.clone(), cli.keypair, args.clone())?,
        Commands::Show(args) => process_show(rpc_client.clone(), args.clone())?,
    }

    Ok(())
}
