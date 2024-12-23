use {
    block_reward_distribution::sdk::{
        derive_config_account_address, derive_reward_distribution_account_address,
        instruction::{
            initialize_ix, initialize_reward_distribution_account_ix, InitializeAccounts,
            InitializeArgs, InitializeRewardDistributionAccountAccounts,
            InitializeRewardDistributionAccountArgs,
        },
    },
    solana_client::rpc_client::RpcClient,
    solana_sdk::{
        commitment_config::CommitmentConfig,
        message::Message,
        pubkey::Pubkey,
        signature::{read_keypair_file, Keypair, Signer},
        system_program,
        transaction::Transaction,
    },
    std::str::FromStr,
};

#[test]
fn init_config_account() {
    // Load Anchor Wallet Keypair
    let anchor_wallet = std::env::var("TEST_WALLET")
        .unwrap_or_else(|_| panic!("Environment variable `TEST_WALLET` is not set"));
    let kp: Keypair = read_keypair_file(&anchor_wallet)
        .unwrap_or_else(|_| panic!("Failed to load keypair from path: `{}`", anchor_wallet));
    println!("✅ Loaded Payer Keypair: {}", kp.pubkey());

    // Load RPC URL
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| panic!("Environment variable `RPC_URL` is not set"));
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
    println!("✅ Connected to RPC URL: {}", rpc_url);

    // Load Program ID
    let tip_distribution_program_id: Pubkey =
        if let Ok(env_program_id) = std::env::var("PROGRAM_ID") {
            env_program_id
                .parse()
                .expect("Invalid PROGRAM_ID in environment")
        } else {
            read_keypair_file("../../target/deploy/block_reward_distribution-keypair.json")
                .expect("Failed to load program ID keypair file")
                .pubkey()
        };
    println!(
        "✅ Tip Distribution Program ID: {}",
        tip_distribution_program_id
    );

    // Derive Config Account Address
    let config_pda_and_bump = derive_config_account_address(&tip_distribution_program_id);
    println!(
        "🛠️ Config Account: {}, Bump: {}",
        config_pda_and_bump.0, config_pda_and_bump.1
    );

    // Create Initialize Instruction
    let initialize_ix = initialize_ix(
        tip_distribution_program_id,
        InitializeArgs {
            authority: kp.pubkey(),
            expired_funds_account: kp.pubkey(),
            num_epochs_valid: 10,
            max_validator_commission_bps: 10_000,
            bump: config_pda_and_bump.1,
        },
        InitializeAccounts {
            config: config_pda_and_bump.0,
            system_program: system_program::id(),
            initializer: kp.pubkey(),
        },
    );

    // Generate Transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .expect("Failed to fetch recent blockhash");
    let message = Message::new(&[initialize_ix], Some(&kp.pubkey()));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    // Send and Confirm Transaction
    let result = rpc_client.send_and_confirm_transaction(&transaction);
    match result {
        Ok(sig) => println!(
            "✅ Config Account Initialized. Transaction Signature: {}",
            sig
        ),
        Err(err) => println!("❌ Failed to Initialize Config Account: {:?}", err),
    }
}

#[test]
fn create_reward_distribution_account() {
    // Load Anchor Wallet Keypair
    let anchor_wallet = std::env::var("TEST_WALLET")
        .unwrap_or_else(|_| panic!("Environment variable `TEST_WALLET` is not set"));
    let kp: Keypair = read_keypair_file(&anchor_wallet)
        .unwrap_or_else(|_| panic!("Failed to load keypair from path: `{}`", anchor_wallet));
    println!("✅ Loaded Payer Keypair: {}", kp.pubkey());

    // Load RPC URL
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| panic!("Environment variable `RPC_URL` is not set"));
    let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), CommitmentConfig::confirmed());
    println!("✅ Connected to RPC URL: {}", rpc_url);

    // Load Program ID
    let tip_distribution_program_id: Pubkey =
        if let Ok(env_program_id) = std::env::var("PROGRAM_ID") {
            env_program_id
                .parse()
                .expect("Invalid PROGRAM_ID in environment")
        } else {
            read_keypair_file("../../target/deploy/block_reward_distribution-keypair.json")
                .expect("Failed to load program ID keypair file")
                .pubkey()
        };
    println!(
        "✅ Tip Distribution Program ID: {}",
        tip_distribution_program_id
    );

    // Derive Config Account Address
    let config_pda_and_bump = derive_config_account_address(&tip_distribution_program_id);
    println!(
        "🛠️ Config Account: {}, Bump: {}",
        config_pda_and_bump.0, config_pda_and_bump.1
    );

    // Load Validator Vote Account
    let vote_account = std::env::var("VOTE_PUBKEY")
        .unwrap_or_else(|_| panic!("Environment variable `VOTE_PUBKEY` is not set"))
        .parse::<Pubkey>()
        .unwrap_or_else(|_| panic!("Failed to parse `VOTE_PUBKEY` as a valid Pubkey"));
    println!("✅ Validator Vote Account Pubkey: {}", vote_account);

    // Derive Reward Distribution Account Address
    let tip_distribution_account = derive_reward_distribution_account_address(
        &tip_distribution_program_id,
        &vote_account,
        rpc_client.get_epoch_info().unwrap().epoch,
    );
    println!(
        "🛠️ Reward Distribution Account: {}, Bump: {:?}",
        tip_distribution_account.0, tip_distribution_account.1
    );

    // Create Initialize Reward Distribution Account Instruction
    let ix = initialize_reward_distribution_account_ix(
        tip_distribution_program_id,
        InitializeRewardDistributionAccountArgs {
            merkle_root_upload_authority: kp.pubkey(),
            validator_commission_bps: 1000,
            bump: tip_distribution_account.1,
        },
        InitializeRewardDistributionAccountAccounts {
            config: config_pda_and_bump.0,
            reward_distribution_account: tip_distribution_account.0,
            system_program: system_program::id(),
            signer: kp.pubkey(),
            validator_vote_account: vote_account,
        },
    );

    // Generate Transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .expect("Failed to fetch recent blockhash");
    let message = Message::new(&[ix], Some(&kp.pubkey()));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    // Send and Confirm Transaction
    let result = rpc_client.send_and_confirm_transaction(&transaction);
    match result {
        Ok(sig) => println!(
            "✅ Reward Distribution Account Created. Transaction Signature: {}",
            sig
        ),
        Err(err) => println!("❌ Failed to Create Reward Distribution Account: {:?}", err),
    }
}
