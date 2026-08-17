# Rakurai CLIs

Command-line tools for Rakurai validator operators and transaction-inclusion partners.

**Audience:** Validator operators, transaction-landing services, and post-pack partners.

---

## 1. Overview

The `rakurai_cli` crate ships four binaries:

| Binary | Audience | Purpose |
| ------ | -------- | ------- |
| `rakurai-activation` | Validator operators | Manage Rakurai Activation Accounts (RAA): init, scheduler control, commission, show |
| `rakurai-revshare` | Transaction-landing / post-pack partners | Partner Tip and MevShare Revenue Settlement — list vaults, record MCA MevShare, settle one or all pending |
| `rakurai-p2c` | P2C User/Consumer | Post-pack prepaid subscription (stake-based fee): fund, record, claim, clear-deficit |
| `rakurai-validator-config` | Rakurai ops / validator operators | Block-engine, P2C (post-pack), and virtual-priority config — global, per-vote, proposals |

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
# Build CLI binaries
cargo b --release -p rakurai_cli

# Export the CLI path
echo "export PATH=\"$(pwd)/target/release/:\$PATH\""
```

### 2.3. Verify installation

```sh
which rakurai-activation
which rakurai-revshare
which rakurai-p2c
which rakurai-validator-config
```

---

## 3. Documentation

| Guide | Description |
| ----- | ----------- |
| [Rakurai Activation CLI](./activation.md) | Initialize and manage Rakurai Activation Accounts (RAA): scheduler enable/disable, commission updates, and account display. |
| [Partner Tip and MevShare Revenue Settlement CLI](./partner_reward_settlement.md) | List TCA/MCA by service, record MCA MevShare (post-pack), settle one vault/epoch or all pending (`rakurai-revshare`). |
| [P2C Subscription CLI](./p2c_subscription.md) | Post-pack (P2C) prepaid subscription for Users/Consumers: stake-based fee, fund → record → claim, unpaid stops P2C, close returns residual (`rakurai-p2c`). |
| [Validator Config CLI](./validator_config.md) | Block-engine, post-pack (P2C), and virtual-priority settings for validators — global, per-vote overlay, operator proposals (`rakurai-validator-config`). |
