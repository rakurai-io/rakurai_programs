# Rakurai Multisig CLI

## Overview

The **Rakurai Multisig CLI** provides a powerful command-line interface to manage a multisig setup for validator operators. It allows users to:
- **Initialize**
- **Update validator commissions**
- **Enable/disable the scheduler**
- **Close**

This tool is designed for **Solana validator operators** who require multisig-based governance for commission updates and related operations.

---

## Installation

Ensure you have **Rust and Cargo** installed before proceeding.

```sh
# Install the CLI tool globally
cargo install --path .
export PATH="$HOME/.cargo/bin:$PATH"

# To check if the CLI is installed correctly:
which rakurai-multisig
```

## Usage

Run the CLI tool with the following command:

```sh
rakurai-multisig [OPTIONS] <COMMAND>
```

### Global Options

- `-k, --keypair <PATH>`: Path to the Solana keypair file (default: `~/.config/solana/id.json`).
- `-r, --rpc <URL>`: RPC URL for sending transactions (default: Testnet).

---

## Commands

### 1. `init-config`

#### Description
Initializes the block builder config account.

#### Usage

```sh
rakurai-multisig init-config [OPTIONS]
```

#### Options

- `-c, --commission_bps <VALUE>`: Block Builder commission percentage in base points (0-10,000).
- `-a, --commission_account <PUBKEY>`: Block builder commission account pubkey.
- `-b, --authority <PUBKEY>`: Block builder multisig authority pubkey.
- `-x, --config_authority <PUBKEY>`: Config account authority pubkey.

#### Example

```sh
rakurai-multisig init-config -c 500 -a <PUBKEY> -b <PUBKEY> -x <PUBKEY>
```

---

### 2. `init-pda`

#### Description
Initializes a new multisig PDA (Program Derived Address) account.

#### Usage

```sh
rakurai-multisig init-pda --commission_bps <VALUE> --identity_pubkey <PUBKEY>
```

#### Options

- `-c, --commission_bps <VALUE>`: Validator commission percentage in base points.
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

#### Example

```sh
rakurai-multisig init-pda -c 500 -v <PUBKEY>
```

---

### 3. `scheduler-control`

#### Description
Enables or disables the scheduler.

#### Usage

```sh
rakurai-multisig scheduler-control [OPTIONS]
```

#### Options

- `-e, --disable_scheduler`: Flag to disable the scheduler (default: enable).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

#### Example

```sh
rakurai-multisig scheduler-control -e iv <PUBKEY>
```

---

### 4. `update-commission`

#### Description
Updates the validator commission.

#### Usage

```sh
rakurai-multisig update-commission [OPTIONS]
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points (optional, if omitted no change is made).
- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

#### Example

```sh
rakurai-multisig update-commission -c 700 -i <PUBKEY>
```

---

### 5. `close`

#### Description
Closes the multisig account.

#### Usage

```sh
rakurai-multisig close --identity_pubkey <PUBKEY>
```

#### Options

- `-i, --identity_pubkey <PUBKEY>`: Validator identity account pubkey.

#### Example

```sh
rakurai-multisig close iv <PUBKEY>
```

---

## Notes

- Ensure your keypair has the necessary permissions to execute transactions.
- Use a valid RPC URL to interact with the Solana blockchain.
- Commission values are in base points (e.g., `500` means `5%`).


