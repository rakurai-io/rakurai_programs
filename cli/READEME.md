# Block Reward Distribution Program

## Overview
The Block Reward Distribution Program is a designed to manage and distribute validator block rewards. The program supports the configuration of key parameters such as reward distribution accounts, validator commission rates, and fund expiration settings.

# Test Case: init_config_account

## Description
This test initializes the configuration account for the block reward distribution program. The configuration account holds settings such as the authority, expiration criteria, expiration account and validator commission limits.

## Required Environment Variables
- `TEST_WALLET`: Path to the keypair file for the payer account.
- `RPC_URL`: RPC endpoint URL.
- `PROGRAM_ID`: Program ID of the deployed block reward distribution program.
  
```bash
TEST_WALLET=~/.config/solana/id.json && RPC_URL=https://api.devnet.solana.com && PROGRAM_ID=ProgramID && cargo test init_config_account -- --nocapture
```
---

# Test Case: create_reward_distribution_account

## Description
This test creates a reward distribution account linked to a specific validator's vote account. The account is used to manage and distribute rewards for the validator.

## Required Environment Variables
- `TEST_WALLET`: Path to the keypair file for the payer account. (keypair linked to vote account)
- `RPC_URL`: RPC endpoint URL.
- `PROGRAM_ID` (Optional): Program ID of the deployed block reward distribution program.
- `VOTE_PUBKEY`: Public key of the validator's vote account.

```bash
TEST_WALLET=~/.config/solana/id.json && RPC_URL=https://api.devnet.solana.com && PROGRAM_ID=ProgramID && VOTE_PUBKEY=ValidatorVotePubkey && cargo test create_reward_distribution_account -- --nocapture
```
---
