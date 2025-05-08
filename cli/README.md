# Rakurai Activation CLI

## Overview

The **Rakurai Activation CLI** provides a command-line interface to manage a activation setup for validator operators. It allows users to:
- **[Initialize](#1-init)**
- **[Update validator commissions](#3-update-commission)**
- **[Enable/disable the scheduler](#2-scheduler-control)**
- **[Show](#4-show)**

---

## Installation

Ensure you have **[Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo)** installed before proceeding.

##### 1. Export the CLI Path
```bash
echo "export PATH=\"$(pwd)/release/downloads:\$PATH\"" >> ~/.bashrc && source ~/.bashrc
```

##### 2. Build from Source
```sh
# Install the CLI tool globally
cargo install --path .
export PATH="$HOME/.cargo/bin:$PATH"

# To check if the CLI is installed correctly:
which rakurai-activation
```

## Usage

Run the CLI tool with the following command:

```sh
rakurai-activation [OPTIONS] <COMMAND>
```

### Global Options

- `-k, --keypair <PATH>`: Path to the Solana keypair file (must be validator identity keypair).
- `-r, --rpc <URL>`: RPC URL for sending transactions (default: Testnet).

---

### Deployed Program ID
- **Testnet**: Recommended for staging and integration testing.
   - *rakurai_activation*: `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt`
- **Mainnet**: Production environment
   - *rakurai_activation*: `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi`

---

### 1. `init`

#### Description
Initializes a new activation Rakurai Activation Account Address (RAA) account.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> init --commission_bps <VALUE> --vote_pubkey <VOTE_PUBKEY> --keypair <IDENTITY_KEYPAIR> --rpc <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: Validator commission percentage in base points.
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

---

### 2. `scheduler-control`

#### Description
Enables or disables the scheduler.

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
Updates the validator commission.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> update-commission --commission_bps <VALUE> --identity_pubkey <IDENTITY_PUBKEY> --keypair <IDENTITY_KEYPAIR> --rpc <RPC_URL>
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points.
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

## Notes

- Ensure your keypair has the necessary permissions to execute transactions.
- Use a valid RPC URL to interact with the Solana blockchain.
- Commission values are in base points (e.g., `500` means `5%`).


