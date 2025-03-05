use {
    anchor_lang::AccountDeserialize,
    clap::{Args, Parser, Subcommand},
    colored::*,
    multisig::{
        sdk::{
            derive_config_account_address, derive_multisig_account_address,
            instruction::{
                initialize_ix, initialize_multi_sig_account_ix, update_multi_sig_approval_ix,
                InitializeAccounts, InitializeArgs, InitializeMultiSigAccountAccounts,
                InitializeMultiSigAccountArgs, UpdateMultiSigApprovalAccounts,
                UpdateMultiSigApprovalArgs,
            },
        },
        state::MultiSigAccount,
    },
    rakurai_cli::{normalize_to_url_if_moniker, parse_keypair, parse_pubkey, validate_commission},
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        message::Message,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
        transaction::Transaction,
        vote::state::VoteStateVersions,
    },
    std::sync::Arc,
};

#[derive(Parser)]
#[command(
    author,
    version,
    about = "A comprehensive CLI tool for managing commission approvals",
    arg_required_else_help = true,
    color = clap::ColorChoice::Always
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to the keypair file (must be a valid Solana keypair file for signing transactions)
    #[arg(short, long, default_value = "~/.config/solana/test_identity.json", value_parser = parse_keypair, help = "Path to the Solana keypair file used for signing transactions")]
    keypair: Arc<Keypair>,

    /// RPC URL for sending transactions
    #[arg(short, long, default_value = "t", value_parser = normalize_to_url_if_moniker, help = "Solana RPC endpoint to send transactions through")]
    rpc: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize block builder config account
    InitConfig(InitConfigArgs),

    /// Initialize a new PDA with a specified commission rate and validator vote account
    InitPda(InitPdaArgs),

    /// Enable or disable the scheduler for commission approvals
    SchedulerControl(SchedulerControlArgs),

    /// Update the commission value for a validator
    UpdateCommission(UpdateCommissionArgs),

    /// Close the commission process, finalizing and deactivating any ongoing commission management
    Close,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = false, color = clap::ColorChoice::Always)]
struct InitConfigArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "Block builder commission percentage in base points (e.g., 500 for 5%)")]
    block_builder_commission_bps: Option<u16>,

    /// Block builder commission account public key
    #[arg(short = 'a', long = "commission_account", value_parser = parse_pubkey, help = "Block builder commission account pubkey")]
    block_builder_commission_account: Option<Pubkey>,

    /// Block builder authority public key
    #[arg(short = 'b', long = "authority", value_parser = parse_pubkey, help = "Block builder multisig authority pubkey")]
    block_builder_authority: Option<Pubkey>,

    /// Config authority public key
    #[arg(short = 'x', long = "config_authority", value_parser = parse_pubkey, help = "Config account authority pubkey")]
    config_authority: Option<Pubkey>,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct InitPdaArgs {
    /// Initial commission percentage in base points (0 to 10,000)
    #[arg(short = 'c', long = "commission_bps", required = true, value_parser = validate_commission, help = "Initial commission percentage in base points (e.g., 500 for 5%)")]
    commission_bps: u16,

    /// Validator vote account public key
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account public key")]
    vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct SchedulerControlArgs {
    /// Enable or disable the scheduler (true = enable, false = disable)
    #[arg(
        short = 'e',
        long = "enable",
        required = true,
        help = "Pass `true` to enable the scheduler, `false` to disable it"
    )]
    enable_scheduler: bool,

    /// Validator vote account public key
    #[arg(short = 'v', long = "vote_pubkey", required = true, value_parser = parse_pubkey, help = "Validator vote account public key")]
    vote_pubkey: Pubkey,
}

#[derive(Args, Clone)]
#[command(arg_required_else_help = true, color = clap::ColorChoice::Always)]
struct UpdateCommissionArgs {
    /// New commission value in base points (0 to 10,000). If omitted, no change is made.
    #[arg(short = 'c', long = "commission_bps", value_parser = validate_commission, help = "New commission value in base points (e.g., 500 for 5%)")]
    commission_bps: Option<u16>,
}

fn process_init_config(rpc_client: Arc<RpcClient>, kp: Arc<Keypair>, args: InitConfigArgs) {
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

    // Fetch Recent Blockhash
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .expect("❌ Failed to fetch recent blockhash");

    // Generate Transaction
    let message = Message::new(&[initialize_instruction], Some(&signer_pubkey));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    // Send and Confirm Transaction
    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✅ Config Account Initialized!\n🔗 Txn Link: {}", sig),
        Err(err) => println!("❌ Failed to Initialize Config Account: {:?}", err),
    }
}

fn process_init_pda(rpc_client: Arc<RpcClient>, kp: Arc<Keypair>, args: InitPdaArgs) {
    let signer_pubkey = kp.pubkey();

    let validator_commission_bps = args.commission_bps;
    let vote_pubkey = args.vote_pubkey;

    let account_info = rpc_client
        .get_account(&vote_pubkey)
        .expect("❌ Failed to fetch vote account info");
    let vote_state = bincode::deserialize::<VoteStateVersions>(&account_info.data)
        .map(|v| v.convert_to_current())
        .expect("❌ Failed to deserialize vote account state");
    if vote_state.node_pubkey != signer_pubkey {
        panic!(
            "❌ Unauthorized signer! Expected: {:?}, Found: {:?}",
            vote_state.node_pubkey, signer_pubkey
        );
    }
    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) = derive_multisig_account_address(&multisig::id(), &vote_pubkey);
    println!("📌 Derived Config PDA: {} (Bump: {})", multisig_pda, bump);
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
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );

    // Fetch Recent Blockhash
    let recent_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(err) => {
            eprintln!("❌ Failed to fetch recent blockhash: {:?}", err);
            return;
        }
    };

    // Generate Transaction
    let message = Message::new(&[initialize_instruction], Some(&signer_pubkey));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    // Send and Confirm Transaction
    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✅ Config Account Initialized!\n🔗 Txn Link: {:?}", sig),
        Err(err) => println!("❌ Failed to Initialize Config Account: {:?}", err),
    }
}

fn process_scheduler_control(
    rpc_client: Arc<RpcClient>,
    kp: Arc<Keypair>,
    args: SchedulerControlArgs,
) {
    let signer_pubkey = kp.pubkey();

    let grant_approval = args.enable_scheduler;
    let vote_pubkey = args.vote_pubkey;

    let (config_pda, _) = derive_config_account_address(&multisig::id());
    let (multisig_pda, bump) = derive_multisig_account_address(&multisig::id(), &vote_pubkey);

    let x = rpc_client.get_account_data(&multisig_pda).unwrap();
    let resp = MultiSigAccount::try_deserialize(&mut x.as_slice()).unwrap();
    println!("{:#?}", resp.block_builder_authority);
    println!("{:#?}", resp.block_builder_commission_account);
    println!("{:#?}", resp.block_builder_commission_bps);
    println!("{:#?}", resp.is_enabled);
    println!("{:#?}", resp.proposer);
    println!("{:#?}", resp.validator_authority);
    println!("{:#?}", resp.validator_commission_bps);
    println!("{:#?}", resp.validator_vote_account);

    println!("📌 Derived Config PDA: {} (Bump: {})", multisig_pda, bump);
    println!(
        "{} {}\n{} {}\n{} {}",
        "🚀 Grant approval:".green(),
        grant_approval,
        "🏦 Vote Pubkey:".blue(),
        vote_pubkey,
        "🔗 Signer:".cyan(),
        signer_pubkey
    );

    let initialize_instruction = update_multi_sig_approval_ix(
        multisig::id(),
        UpdateMultiSigApprovalArgs { grant_approval },
        UpdateMultiSigApprovalAccounts {
            config: config_pda,
            validator_vote_account: vote_pubkey,
            multisig_account: multisig_pda,
            signer: signer_pubkey,
        },
    );

    // Fetch Recent Blockhash
    let recent_blockhash = match rpc_client.get_latest_blockhash() {
        Ok(hash) => hash,
        Err(err) => {
            eprintln!("❌ Failed to fetch recent blockhash: {:?}", err);
            return;
        }
    };

    // Generate Transaction
    let message = Message::new(&[initialize_instruction], Some(&signer_pubkey));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    // Send and Confirm Transaction
    match rpc_client.send_and_confirm_transaction(&transaction) {
        Ok(sig) => println!("✅ Config Account Initialized!\n🔗 Txn Link: {:?}", sig),
        Err(err) => println!("❌ Failed to Initialize Config Account: {:?}", err),
    }

    let x = rpc_client.get_account_data(&multisig_pda).unwrap();
    let resp = MultiSigAccount::try_deserialize(&mut x.as_slice()).unwrap();
    println!("{:#?}", resp.block_builder_authority);
    println!("{:#?}", resp.block_builder_commission_account);
    println!("{:#?}", resp.block_builder_commission_bps);
    println!("{:#?}", resp.is_enabled);
    println!("{:#?}", resp.proposer);
    println!("{:#?}", resp.validator_authority);
    println!("{:#?}", resp.validator_commission_bps);
    println!("{:#?}", resp.validator_vote_account);
}

fn process_update_commission(args: UpdateCommissionArgs) {
    if let Some(commission) = args.commission_bps {
        println!("{} {}", "🔄 Updating commission to:".yellow(), commission);
    } else {
        println!(
            "{}",
            "⚠️ No commission value provided. No changes made.".red()
        );
    }
}

fn process_close() {
    println!("{}", "🔒 Closing commission process".red());
}

fn main() {
    let cli = Cli::parse();

    // RPC client
    let rpc_client = Arc::new(RpcClient::new_with_commitment(
        cli.rpc.clone(),
        CommitmentConfig::confirmed(),
    ));
    match &cli.command {
        Commands::InitPda(args) => process_init_pda(rpc_client, cli.keypair, args.clone()),
        Commands::InitConfig(args) => process_init_config(rpc_client, cli.keypair, args.clone()),
        Commands::SchedulerControl(args) => {
            process_scheduler_control(rpc_client, cli.keypair, args.clone())
        }
        Commands::UpdateCommission(args) => process_update_commission(args.clone()),
        Commands::Close => process_close(),
    }
}
