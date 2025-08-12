# Rakurai Activation CLI

## Overview

The **Rakurai Activation CLI** is a CLI tool to manage the Rakurai activation setup for validator operators. It interacts with a **multisig-based Solana smart contract** responsible for enabling or disabling the Rakurai scheduler.  
Each validator must create a **Rakurai Activation Account (RAA)** — a PDA jointly controlled by both the validator and Rakurai. Enabling the Rakurai scheduler requires **2/2 multisig approval** (both validator and Rakurai). Disabling the scheduler can be done **unilaterally by either party (1/2 multisig)**. Validators can also configure the percentage of block rewards they want to retain (0–100%). The remaining portion is distributed to stakers.

- **[Initialize RAA](#1-init)**
- **[Update commissions](#3-update-commission)**
- **[Enable/disable the scheduler](#2-scheduler-control)**
- **[Display RAA State](#4-show)**

---

## Installation

Ensure you have **[Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo)** installed before proceeding.

You can either **build from source** or use the **prebuilt binary from the `release/downloads` directory**.

### Option 1: Use Prebuilt CLI
```bash
# Export the prebuilt CLI binary to your PATH
echo "export PATH=\"$(pwd)/release/downloads:\$PATH\"" >> ~/.bashrc && source ~/.bashrc
```

### Option 2: Build from Source
```sh
# Build and install the CLI tool globally
cargo install --path .
export PATH="$HOME/.cargo/bin:$PATH"
```

### Verify Installation
```sh
which rakurai-activation
```

---

## Usage

Run the CLI tool using:

```sh
rakurai-activation [OPTIONS] <COMMAND>
```

---

### Global Options

These options are **critical** and must be used with care:

- `-k, --keypair <PATH>`: Path to the Solana keypair file. This keypair must have authority to send transactions—typically the **validator identity keypair**.
- `-u, --url <URL>`: RPC URL of the target Solana cluster or moniker (for testnet cluster use `-ut` or `-u https://api.testnet.solana.com`).  
- `-p, --program-id <PROGRAM_ID>`: Deployed Rakurai Activation program ID.  
  - **Mainnet:** `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi`  
  - **Testnet:** `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt`

> ❗ Incorrect keypair, RPC, or program ID will result in failed transactions.

---

### 1. `init`

#### Description

To create a Rakurai Activation Account (RAA), you must provide your validator's `vote_pubkey` along with its corresponding *node identity* `keypair` as the signer. This node identity will be used later to authorize or modify RAA parameters.
You must also specify a `commission_bps` in basis points (BPS), which defines the validator’s share of the block rewards. This commission is **independent** of Solana’s voting commission and applies only to block rewards.
Creating an RAA is a **mandatory step** for running a Rakurai-Solana validator


#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> init --commission_bps <VALUE> --vote_pubkey <VOTE_PUBKEY> --keypair <IDENTITY_KEYPAIR> --url <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: Validator's commission on block rewards in basis points (e.g., `500` for `5%`).
- `-v, --vote_pubkey <PUBKEY>`: Validator's vote account public key.

---

### 2. `scheduler-control`

#### Description
Controls the Rakurai scheduler by enabling or disabling it.

- To **enable** the scheduler, both the validator and Rakurai must independently submit a transaction. The scheduler becomes active **only after both sides** have performed the enable transaction.
- To **disable** the scheduler, **either party** (the validator or Rakurai) can submit a disable transaction. Only one side is required to disable it.
- By default, this command **enables** the scheduler. To explicitly disable it, use the `-d` or `--disable_scheduler` flag.
  
#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> scheduler-control --identity_pubkey <IDENTITY_PUBKEY> --keypair <IDENTITY_KEYPAIR> --url <RPC_URL> 
```

#### Options

- `-d, --disable_scheduler`: Flag to disable the scheduler (default: enable).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---

### 3. `update-commission`

#### Description
Updates the validator's block reward commission.

Validators can update their share of the block reward at any time, independent of Rakurai. Only the validator can change their commission, and the change will take effect starting from the next epoch. However, if the validator has not yet passed the first leader turn of the current epoch, the new commission will be applied in the following epoch.
 
#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> update-commission --commission_bps <VALUE> --identity_pubkey <IDENTITY_PUBKEY> --keypair <IDENTITY_KEYPAIR> --url <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points (e.g., `500` means `5%`).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---

### 4. `show`

#### Description
Displays the current state of the Rakurai Activation Account (RAA).

- **Scheduler status**: Whether the Rakurai scheduler is enabled or disabled.
- **Validator commission**: The current block reward commission in basis points.
- **Authority**: The account authorized to manage the Rakurai Activation Account (RAA).

This command provides an overview of the RAA's current configuration and state.


#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> show --identity_pubkey <IDENTITY_PUBKEY> --url <RPC_URL>
```

#### Options

- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---
