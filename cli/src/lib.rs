use {
    anchor_lang::AccountDeserialize,
    colored::*,
    rakurai_activation::state::{RakuraiActivationAccount, RakuraiActivationConfigAccount},
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        instruction::Instruction,
        message::Message,
        pubkey::Pubkey,
        signature::Keypair,
        signer::{EncodableKey, Signer},
        transaction::Transaction,
    },
    solana_vote_interface::state::{VoteStateV3, VoteStateVersions},
    std::{path::Path, str::FromStr, sync::Arc},
};

/// Parses and validates a Solana `Pubkey` from a string
pub fn parse_pubkey(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|_| format!("Invalid Solana public key: {}", s))
}

/// Normalizes an RPC URL or moniker to a valid Solana RPC endpoint
pub fn normalize_to_url_if_moniker(url_or_moniker: &str) -> Result<String, String> {
    let url = match url_or_moniker.as_ref() {
        "m" | "mainnet-beta" => "https://api.mainnet-beta.solana.com",
        "t" | "testnet" => "https://api.testnet.solana.com",
        "d" | "devnet" => "https://api.devnet.solana.com",
        "l" | "localhost" => "http://localhost:8899",
        url => url,
    };
    Ok(url.to_string())
}

/// Validates that commission is between 0 and 10,000
pub fn validate_commission(val: &str) -> Result<u16, String> {
    val.parse::<u16>()
        .map_err(|_| "Commission must be a valid positive integer".to_string())
        .and_then(|v| {
            if v <= 10_000 {
                Ok(v)
            } else {
                Err("Commission must be between 0 and 10,000 (0% to 100%)".to_string())
            }
        })
}

/// Parses a Solana keypair from a file
pub fn parse_keypair(path: &str) -> Result<Arc<Keypair>, Box<dyn std::error::Error>> {
    let expanded_path = shellexpand::tilde(path).into_owned();
    let path = Path::new(&expanded_path);
    if !path.exists() {
        return Err(format!(
            "❌ Keypair file not found: {}. Please provide a valid keypair path. (--keypair path/to/keypair.json)", 
            expanded_path
        )
        .into());
    }
    Keypair::read_from_file(path)
        .map(Arc::new)
        .map_err(|e| format!("Failed to read keypair from file {}: {}", expanded_path, e).into())
}

pub fn get_activation_account(
    rpc_client: Arc<RpcClient>,
    activation_pda: Pubkey,
) -> Result<RakuraiActivationAccount, Box<dyn std::error::Error>> {
    let account_data = rpc_client.get_account_data(&activation_pda)?;
    let mut account_slice = account_data.as_slice();
    RakuraiActivationAccount::try_deserialize(&mut account_slice).map_err(Into::into)
}

pub fn get_activation_config_account(
    rpc_client: Arc<RpcClient>,
    activation_config_account: Pubkey,
) -> Result<RakuraiActivationConfigAccount, Box<dyn std::error::Error>> {
    let account_data = rpc_client.get_account_data(&activation_config_account)?;
    let mut account_slice = account_data.as_slice();
    RakuraiActivationConfigAccount::try_deserialize(&mut account_slice).map_err(Into::into)
}

pub fn display_activation_account(activation_account: RakuraiActivationAccount) {
    println!("{}", "🗳️ Validator".bold().underline().blue());
    println!(
        "   {} {:<10} {}",
        "✅".green(),
        "Enabled:",
        activation_account.is_enabled.to_string().blue()
    );
    println!(
        "   {} {:<10} {}",
        "💰".green(),
        "Commission:",
        activation_account
            .validator_commission_bps
            .to_string()
            .magenta()
    );
    println!(
        "   {} {:<10} {}",
        "🔑".red(),
        "Authority:",
        activation_account.validator_authority.to_string()
    );

    if let Some(proposer) = activation_account.proposer {
        println!("{}", "📝 Proposer".bold().underline().blue());
        println!(
            "   {} {:<10} {}",
            "📝".cyan(),
            "Proposer:",
            proposer.to_string()
        );
    }
    if let Some(array) = activation_account.hash {
        println!("{}", "📝 Hash".bold().underline().blue());
        println!(
            "   {} {:<10} {}",
            "📝".cyan(),
            "Hash:",
            bs58::encode(array).into_string()
        );
    }
}

pub fn display_activation_config_account(
    activation_config_account: RakuraiActivationConfigAccount,
) {
    println!(
        "{}",
        "📜 Activation Config Account".bold().underline().blue()
    );
    println!(
        "   {} {:<10} {}",
        "💰".green(),
        "Commission:",
        activation_config_account
            .block_builder_commission_bps
            .to_string()
            .magenta()
    );
    println!(
        "   {} {:<10} {}",
        "🏦".cyan(),
        "Commission Account:",
        activation_config_account
            .block_builder_commission_account
            .to_string()
    );
    println!(
        "   {} {:<10} {}",
        "🔑".red(),
        "Authority:",
        activation_config_account
            .block_builder_authority
            .to_string()
            .magenta()
    );
}

pub fn get_vote_account(
    rpc_client: Arc<RpcClient>,
    vote_pubkey: Pubkey,
) -> Result<VoteStateV3, Box<dyn std::error::Error>> {
    let account_info = rpc_client.get_account(&vote_pubkey)?;
    let vote_state_versions: VoteStateVersions = bincode::deserialize(&account_info.data)?;
    Ok(vote_state_versions.convert_to_v3())
}

pub fn sign_and_send_transaction(
    rpc_client: Arc<RpcClient>,
    instruction: Instruction,
    signer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    match rpc_client.get_latest_blockhash() {
        Ok(hash) => {
            let transaction = Transaction::new(
                &[&signer],
                Message::new(&[instruction], Some(&signer.pubkey())),
                hash,
            );
            match rpc_client.send_and_confirm_transaction(&transaction) {
                Ok(sig) => {
                    println!("✅ Transaction Confirmed \n🔗 Txn Signature: {:?}", sig);
                    Ok(())
                }
                Err(err) => Err(err.into()),
            }
        }
        Err(err) => {
            return Err(err.into());
        }
    }
}
