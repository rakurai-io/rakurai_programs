#!/bin/bash

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <vote-account-pubkey> <identity-keypair>"
    exit 1
fi

VOTE_ACCOUNT_PUBKEY=$(solana-keygen pubkey $1)
IDENTITY_KEYPAIR=$2
RPC_URL="http://127.0.0.1:8899"
AUTHORITY_KEYPAIR_FILE="rakurai_activation_authority.json"

# Generate authority keypair
solana-keygen new --silent --no-bip39-passphrase --outfile "$AUTHORITY_KEYPAIR_FILE"
AUTHORITY_KEYPAIR_PUBKEY=$(solana-keygen pubkey "$AUTHORITY_KEYPAIR_FILE")

echo "🔹 Using Authority Pubkey: $AUTHORITY_KEYPAIR_PUBKEY"

echo "💰 Airdropping SOL to authority..."
solana airdrop 20 "$AUTHORITY_KEYPAIR_PUBKEY" --url "$RPC_URL"

echo "🚀 Building Anchor Program..."
anchor build --program-name rakurai_activation
anchor keys sync
anchor build --program-name rakurai_activation

echo "🚀 Deploying Anchor Program..."
anchor deploy --provider.cluster "$RPC_URL" --provider.wallet "$AUTHORITY_KEYPAIR_FILE"  --program-name rakurai_activation

# Build CLI
echo "🛠️ Building CLI..."
cargo install --path cli --bin rakurai-activation

echo "🔑 Initializing Activation Config Account..."
rakurai-activation init-config --commission_bps 500 --authority $AUTHORITY_KEYPAIR_PUBKEY -r "$RPC_URL" -k "$AUTHORITY_KEYPAIR_FILE"

echo "✅ Setup complete!"
# rm "$AUTHORITY_KEYPAIR_FILE"
