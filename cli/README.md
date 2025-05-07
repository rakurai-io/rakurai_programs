# Rakurai Activation CLI

## Overview

The **Rakurai Activation CLI** provides a command-line interface to manage a activation setup for validator operators. It allows users to:
- **Initialize**
- **Update validator commissions**
- **Enable/disable the scheduler**

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

### 1. `init`

#### Description
Initializes a new activation PDA (Program Derived Address) account.

#### Usage

```sh
rakurai-activation -p <PROGRAM_ID> init-pda --commission_bps <VALUE> --identity_pubkey <PUBKEY>
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
rakurai-activation scheduler-control [OPTIONS]
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
rakurai-activation update-commission [OPTIONS]
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points (optional, if omitted no change is made).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

---

## Notes

- Ensure your keypair has the necessary permissions to execute transactions.
- Use a valid RPC URL to interact with the Solana blockchain.
- Commission values are in base points (e.g., `500` means `5%`).


