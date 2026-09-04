use {
    anchor_lang::{prelude::Pubkey as AnchorPubkey, AccountDeserialize},
    bincode::deserialize,
    colored::*,
    rakurai_activation::state::{RakuraiActivationAccount, RakuraiActivationConfigAccount},
    solana_instruction::{AccountMeta, Instruction},
    solana_keypair::{read_keypair_file, Keypair},
    solana_message::Message,
    solana_pubkey::Pubkey,
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_transaction::Transaction,
    std::{
        io::{self, Write},
        path::Path,
        str::FromStr,
        sync::Arc,
    },
};

pub const MAX_COMMISSION_BPS: u16 = 10_000;

pub mod validator;

/// Convert workspace `Pubkey` to Anchor / `solana-program` 2 `Pubkey`.
pub fn to_anchor_pubkey(pubkey: Pubkey) -> AnchorPubkey {
    AnchorPubkey::new_from_array(*pubkey.as_array())
}

/// Convert Anchor / `solana-program` 2 `Pubkey` to workspace `Pubkey`.
pub fn from_anchor_pubkey(pubkey: AnchorPubkey) -> Pubkey {
    Pubkey::new_from_array(pubkey.to_bytes())
}

/// Rebuild an Anchor program instruction as a workspace `solana-instruction` value.
pub fn to_solana_instruction(
    mut ix: anchor_lang::solana_program::instruction::Instruction,
) -> Instruction {
    let acct_metas: Vec<AccountMeta> = ix
        .accounts
        .iter_mut()
        .map(|acct| AccountMeta {
            pubkey: Pubkey::new_from_array(acct.pubkey.to_bytes()),
            is_signer: acct.is_signer,
            is_writable: acct.is_writable,
        })
        .collect();
    Instruction::new_with_bytes(
        Pubkey::new_from_array(ix.program_id.to_bytes()),
        &ix.data,
        acct_metas,
    )
}

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
    read_keypair_file(path)
        .map(Arc::new)
        .map_err(|e| format!("Failed to read keypair from file {}: {}", expanded_path, e).into())
}

pub fn reconfirm_commission(
    block_reward_commission_bps: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n {}", 
        format!(
            "⚠ Note: Validator Block Reward Commission set to {} bps ({}%). Validator will keep {}% of the block rewards. The remaining {}% will be distributed among stakers.",
            block_reward_commission_bps,
            block_reward_commission_bps as f64 / 100.0,
            block_reward_commission_bps as f64 / 100.0,
            (MAX_COMMISSION_BPS - block_reward_commission_bps) as f64 / 100.0,
        )
        .red()
    );

    println!("Type '{}' to confirm:", block_reward_commission_bps);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let confirm_value = validate_commission(input.trim())?;

    if confirm_value != block_reward_commission_bps {
        return Err(format!(
            "❌ Commission confirmation mismatch!. Expected: {} bps, Entered: {} bps. Aborting Please re-run the command with correct commission value.", 
            block_reward_commission_bps,confirm_value
        )
        .into());
    }
    Ok(())
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
        "   {} {:<10} {} ({}%) -> {}",
        "💰".green(),
        "Commission:",
        activation_account
            .block_reward_commission_bps
            .to_string()
            .magenta(),
        (activation_account.block_reward_commission_bps as f64 / 100.0),
        if activation_account.block_reward_commission_bps == MAX_COMMISSION_BPS {
            "Validator will keep 100% of the block rewards. No rewards will be distributed to stakers."
                .green()
                .to_string()
        } else {
            format!(
                "{} Validator will keep {}% of the block rewards. The remaining {}% will be distributed among stakers.",
                "Note:".yellow().bold(),
                (activation_account.block_reward_commission_bps as f64 / 100.0),
                ((MAX_COMMISSION_BPS - activation_account.block_reward_commission_bps) as f64 / 100.0),
            )
        },
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
            .client_commission_bps
            .to_string()
            .magenta()
    );
    println!(
        "   {} {:<10} {}",
        "🏦".cyan(),
        "Commission Account:",
        activation_config_account
            .client_commission_account
            .to_string()
    );
    println!(
        "   {} {:<10} {}",
        "🔑".red(),
        "Authority:",
        activation_config_account
            .client_authority
            .to_string()
            .magenta()
    );
}

pub fn get_node_pubkey_from_vote_account(
    rpc_client: Arc<RpcClient>,
    vote_pubkey: Pubkey,
) -> Result<Pubkey, Box<dyn std::error::Error>> {
    let account_info = rpc_client.get_account(&vote_pubkey)?;
    deserialize::<Pubkey>(&account_info.data[4..36])
        .map_err(|e| format!("Failed to deserialize node pubkey from vote account: {}", e).into())
}

pub fn sign_and_send_transaction(
    rpc_client: Arc<RpcClient>,
    instruction: Instruction,
    signer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    sign_and_send_instructions(rpc_client, std::slice::from_ref(&instruction), signer)
}

/// Signs and confirms a transaction with one or more instructions.
pub fn sign_and_send_instructions(
    rpc_client: Arc<RpcClient>,
    instructions: &[Instruction],
    signer: &Keypair,
) -> Result<(), Box<dyn std::error::Error>> {
    if instructions.is_empty() {
        return Err("no instructions to send".into());
    }
    match rpc_client.get_latest_blockhash() {
        Ok(hash) => {
            let transaction = Transaction::new(
                &[&signer],
                Message::new(instructions, Some(&signer.pubkey())),
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
        Err(err) => Err(err.into()),
    }
}
