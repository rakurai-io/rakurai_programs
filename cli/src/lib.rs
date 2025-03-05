use {
    solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::EncodableKey},
    std::{str::FromStr, sync::Arc},
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
pub fn validate_commission(val: &str) -> Result<u64, String> {
    val.parse::<u64>()
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
pub fn parse_keypair(path: &str) -> Result<Arc<Keypair>, String> {
    let expanded_path = shellexpand::tilde(path).into_owned();
    Keypair::read_from_file(&expanded_path)
        .map(Arc::new)
        .map_err(|_| format!("Invalid keypair format in file: {}", expanded_path))
}
