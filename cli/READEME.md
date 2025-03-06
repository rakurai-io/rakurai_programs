# Multisig CLI

## Overview

This CLI tool provides a comprehensive interface for managing rakurai multisig setup. validator operators can initialize, update commissions, enable or disable schedulers, and close accounts.

## Installation

Ensure you have Rust and Cargo installed before using this CLI tool.

```sh
# Install CLI tool 
cargo install --path .
```

## Usage

Run the CLI tool with the following command:

```sh
rakurai-multisig-cli [OPTIONS] <COMMAND>
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
rakurai-multisig-cli init-config [OPTIONS]
```

#### Options

- `-c, --commission_bps <VALUE>`: Block Builder commission percentage in base points (0-10,000).
- `-a, --commission_account <PUBKEY>`: Block builder commission account pubkey.
- `-b, --authority <PUBKEY>`: bBlock builder multisig authority pubkey.
- `-x, --config_authority <PUBKEY>`: Config account authority pubkey.

#### Example

```sh
rakurai-multisig-cli init-config -c 500 -a <PUBKEY> -b <PUBKEY> -x <PUBKEY>
```

---

### 2. `init-pda`

#### Description
Initializes a new multisig PDA (Program Derived Address) account.

#### Usage

```sh
rakurai-multisig-cli init-pda --commission_bps <VALUE> --vote_pubkey <PUBKEY>
```

#### Options

- `-c, --commission_bps <VALUE>`: Validator commission percentage in base points.
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

#### Example

```sh
rakurai-multisig-cli init-pda -c 500 -v <PUBKEY>
```

---

### 3. `scheduler-control`

#### Description
Enables or disables the scheduler.

#### Usage

```sh
rakurai-multisig-cli scheduler-control [OPTIONS]
```

#### Options

- `-e, --disable_scheduler`: Flag to disable the scheduler (default: enable).
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

#### Example

```sh
rakurai-multisig-cli scheduler-control -e -v <PUBKEY>
```

---

### 4. `update-commission`

#### Description
Updates the validator commission.

#### Usage

```sh
rakurai-multisig-cli update-commission [OPTIONS]
```

#### Options

- `-c, --commission_bps <VALUE>`: New commission value in base points (optional, if omitted no change is made).
- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

#### Example

```sh
rakurai-multisig-cli update-commission -c 700 -v <PUBKEY>
```

---

### 5. `close`

#### Description
Closes the multisig account.

#### Usage

```sh
rakurai-multisig-cli close --vote_pubkey <PUBKEY>
```

#### Options

- `-v, --vote_pubkey <PUBKEY>`: Validator vote account pubkey.

#### Example

```sh
rakurai-multisig-cli close -v <PUBKEY>
```

---

## Notes

- Ensure your keypair has the necessary permissions to execute transactions.
- Use a valid RPC URL to interact with the Solana blockchain.
- Commission values are in base points (e.g., `500` means `5%`).


