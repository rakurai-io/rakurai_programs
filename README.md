# rakurai_programs

anchor build
anchor  keys sync
anchor build
anchor deploy --provider.cluster t --provider.wallet ../txn-generator-client/staked-identity-copy.json
anchor upgrade --program-id 73rhHmGAfK4E7KEaTz7PxStSUicuRx4JcKN1tUC4x1Ev ./target/deploy/block_reward_distribution.so   --provider.cluster t --provider.wallet ../txn-generator-client/staked-identity-copy.json 
solana program close 4wyjfWEX6746eoepd37Gb6KcPpLpkJhe4CqWzerLfpCB --keypair ../txn-generator-client/staked-identity-copy.json  -ut --bypass-warning


cargo test -- --nocapture
cargo test init_config_account -- --nocapture
cargo test create_reward_distribution_account -- --nocapture
