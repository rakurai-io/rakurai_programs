# Rakurai CLIs

Command-line tools for Rakurai validator operators and transaction-inclusion partners.

**Audience:** Validator operators, searchers, block engines, and post-pack / MEV partners.

---

## 1. Overview

The `rakurai_cli` crate ships two binaries:

| Binary | Audience | Purpose |
| ------ | -------- | ------- |
| `rakurai-activation` | Validator operators | Manage Rakurai Activation Accounts (RAA): init, scheduler control, commission, show |
| `rakurai-partner-reward-settlement` | Custom tip / post-pack partners | Inspect TCA/MCA vaults, list pending epoch records, and settle SOL |

---

## 2. Installation

Ensure you have **[Rust and Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html#install-rust-and-cargo)** installed before proceeding.

You can either **build from source** or use the **prebuilt binary from the `release/downloads` directory**.

### 2.1. Option 1: Use prebuilt CLI

```bash
# Export the prebuilt CLI binary to your PATH
echo "export PATH=\"$(pwd)/release/downloads:\$PATH\"" >> ~/.bashrc && source ~/.bashrc
```

### 2.2. Option 2: Build from source

```sh
# Build both CLI binaries
cargo b --release -p rakurai_cli

# Export the CLI path
echo "export PATH=\"$(pwd)/target/release/:\$PATH\""
```

### 2.3. Verify installation

```sh
which rakurai-activation
which rakurai-partner-reward-settlement
```

---

## 3. Documentation

| Guide | Description |
| ----- | ----------- |
| [Rakurai Activation CLI](./ACTIVATION.md) | Initialize and manage Rakurai Activation Accounts (RAA): scheduler enable/disable, commission updates, and account display. |
| [Partner Reward Settlement CLI](./PARTNER_REWARD_SETTLEMENT.md) | Locate TCA/MCA vaults, inspect pending epoch records, and settle custom tip or post-pack/MEV revenue (legacy and V1). |
