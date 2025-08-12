# Rakurai Programs

This repository contains Solana smart contracts and tools for managing Rakurai's validator operations:

- **Rakurai Activation**  
  A multisig-based smart contract that enables and controls validators to run a Rakurai node.
  ➤ For more details, refer to the [README](./programs/rakurai_activation/README.md).

- **Reward Distribution**  
  A smart contract for distributing block rewards to stakers using a permissionless, Merkle-root-based verification model.  
  ➤ For more details, refer to the [README](./programs/reward_distribution/README.md).

- **Rakurai Activation CLI**  
  A command-line interface for proposing and approving multisig actions related to the **Rakurai Activation** smart contract.  
  ➤ For more details, refer to the [README](./cli/README.md).


## How to build and deploy programs using Anchor
This repository contains the necessary commands and steps for building, deploying, upgrading, and closing Solana programs. The commands utilize the Anchor framework for managing the deployment and interaction with Solana-based programs.

## Setup & Installation

1. **Install Anchor**: 
   Make sure you have Anchor installed. If not, follow the official instructions to install it: 
   [Install Anchor](https://project-serum.github.io/anchor/getting-started/installation.html).

2. **Install Solana CLI**: 
   Ensure that the Solana CLI is installed. Follow the official installation guide: 
   [Solana CLI Installation](https://docs.solana.com/cli/install-solana-cli-tools).

## Commands Overview

### 1. Build the Anchor Program
To build the program, use the following command:

```bash
anchor build
```

### 1. Sync the Wallet Keys
Synchronize Program ID and rebuild:

```bash
anchor keys sync
anchor build
```
### 3. Deploy the Program
To deploy the program to the Solana cluster, use the following command. Make sure to specify the correct wallet and program files:

```bash
anchor deploy --provider.cluster t --provider.wallet ~/.config/solana/id.json
```
This will deploy your program to the Solana testnet (t cluster) using the wallet at `~/.config/solana/id.json`.

### 4. Upgrade the Program
To upgrade an existing deployed program, use the following command. Replace the program-id with your deployed program ID and specify the path to the new program .so file:

```bash
anchor upgrade --program-id <DeloyedProgramID> ./target/deploy/block_reward_distribution.so --provider.cluster t --provider.wallet ~/.config/solana/id.json
```
This upgrades the program at the specified program-id to the new program located at ./target/deploy/block_reward_distribution.so.

### 5. Close the Program
To close a Solana program, use the following command. You must provide the program's public key and the path to your wallet's keypair file:

```bash
solana program close 4wyjfWEX6746eoepd37Gb6KcPpLpkJhe4CqWzerLfpCB --keypair ~/.config/solana/id.json -ut --bypass-warning
```
This command will close the program and reclaim the funds.