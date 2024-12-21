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
    // Keypair loading
    let keypair_path = "../../txn-generator-client/staked-identity-copy.json";
    let kp: Keypair = read_keypair_file(keypair_path).expect("Failed to load keypair");

    println!("Keypair loaded successfully!");
    println!("Public Key: {}", kp.pubkey());

    // RPC setup
    let rpc_url = "https://api.testnet.solana.com";
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Program ID and Config Account
    let tip_distribution_program_id: Pubkey = read_keypair_file(
        "/home/ubuntu/rakurai_programs/target/deploy/block_reward_distribution-keypair.json",
    )
    .expect("Failed to read program keypair")
    .pubkey();
    println!("Tip Program Address: {}", tip_distribution_program_id);

    let config_pda_and_bump = derive_config_account_address(&tip_distribution_program_id);
    println!("Config Account: {}", config_pda_and_bump.0);
    println!("Config Bump Seed: {}", config_pda_and_bump.1);

    // Create initialize instruction
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

    // Generate transaction
    let recent_blockhash = rpc_client
        .get_latest_blockhash()
        .expect("Failed to fetch blockhash");
    let message = Message::new(&[initialize_ix], Some(&kp.pubkey()));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    println!("Signature: {:?}", transaction.signatures[0]);
    let result = rpc_client
        .simulate_transaction(&transaction)
        .expect("Failed to confirm transaction");

    println!("Config Account Result: {:?}", result);
}

#[test]
fn create_reward_distribution_account() {
    let keypair_path = "../../txn-generator-client/staked-identity-copy.json";
    let kp: Keypair = read_keypair_file(keypair_path).expect("Failed to load keypair");

    println!("Keypair loaded successfully!");
    println!("Public Key: {}", kp.pubkey());

    // RPC setup
    let rpc_url = "https://api.testnet.solana.com";
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Program ID and Config Account
    // Program ID and Config Account
    let tip_distribution_program_id: Pubkey = read_keypair_file(
        "/home/ubuntu/rakurai_programs/target/deploy/block_reward_distribution-keypair.json",
    )
    .expect("Failed to read program keypair")
    .pubkey();
    println!("Tip Program Address: {}", tip_distribution_program_id);

    let config_pda_and_bump = derive_config_account_address(&tip_distribution_program_id);
    println!("Config Account: {}", config_pda_and_bump.0);
    println!("Config Bump Seed: {}", config_pda_and_bump.1);

    // TDA Account for each epoch
    let keypair_path = "../../txn-generator-client/staked-identity-copy.json";
    let kp: Keypair = read_keypair_file(keypair_path).unwrap();
    let vote_account = Pubkey::from_str("6No5zMEn7SYhrBLQe47sprXzzhNPW2UW1nHzm4dbC1df").unwrap();
    let (tip_distribution_account, bump) = derive_reward_distribution_account_address(
        &tip_distribution_program_id,
        &vote_account,
        rpc_client.get_epoch_info().unwrap().epoch,
    );
    println!("TDA Account: {}", tip_distribution_account);
    println!("TDA Bump Seed: {}", bump);
    let ix = initialize_reward_distribution_account_ix(
        tip_distribution_program_id,
        InitializeRewardDistributionAccountArgs {
            merkle_root_upload_authority: kp.pubkey(),
            validator_commission_bps: 1000,
            bump,
        },
        InitializeRewardDistributionAccountAccounts {
            config: config_pda_and_bump.0,
            reward_distribution_account: tip_distribution_account,
            system_program: system_program::id(),
            signer: kp.pubkey(),
            validator_vote_account: vote_account,
        },
    );
    let recent_blockhash = rpc_client.get_latest_blockhash().unwrap();
    let message = Message::new(&[ix], Some(&kp.pubkey()));
    let transaction = Transaction::new(&[&kp], message, recent_blockhash);

    println!("signature {:?}", transaction.signatures[0]);
    let result = rpc_client
        .simulate_transaction(&transaction)
        .expect("Failed to confirm transaction");

    println!("Config Account result: {:?}", result);
}
