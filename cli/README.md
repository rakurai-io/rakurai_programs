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
- `-r, --rpc <URL>`: RPC URL of the target Solana cluster or moniker (for testnet cluster use `-rt` or `-r https://api.testnet.solana.com`).  
- `-p, --program-id <PROGRAM_ID>`: Deployed Rakurai Activation program ID.  
  - **Mainnet:** `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi`  
  - **Testnet:** `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt`

> ❗ Incorrect keypair, rpc, or program ID will result in failed transactions.

---

### 1. `init`

#### Description
Initializes a new Rakurai Activation Account (RAA).

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> init --commission_bps <VALUE> --vote_pubkey <VOTE_PUBKEY> --keypair <IDENTITY_KEYPAIR> --rpc <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: Validator block rewards commission percentage in BPS (e.g., `500` means `5%`).
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

---

### 2. `scheduler-control`

#### Description
Enables or Disables the scheduler.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> scheduler-control --identity_pubkey <IDENTITY_PUBKEY> --keypair <IDENTITY_KEYPAIR> --rpc <RPC_URL> 
```

#### Options

- `-d, --disable_scheduler`: Flag to disable the scheduler (default: enable).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---

### 3. `update-commission`

#### Description
Updates the validator block reward commission.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> update-commission --commission_bps <VALUE> --identity_pubkey <IDENTITY_PUBKEY> --keypair <IDENTITY_KEYPAIR> --rpc <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points (e.g., `500` means `5%`).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---

### 4. `show`

#### Description
Display the rakurai activation account state.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> show --identity_pubkey <IDENTITY_PUBKEY> --rpc <RPC_URL>
```

#### Options

- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---
