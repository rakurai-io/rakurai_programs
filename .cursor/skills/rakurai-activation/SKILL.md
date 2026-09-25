---
name: rakurai-activation
description: >-
  Rakurai Activation on-chain program (RAA, config PDA), multisig scheduler
  enable/disable, and validator commission settings. Use for rakurai activation,
  RAA, scheduler control, validator commission, client authority, or the
  rakurai-activation CLI.
---

# Rakurai Activation

`programs/rakurai_activation/`. Gates Rakurai scheduler use; stores per-validator commission. **RAA** PDA per validator identity; singleton **config PDA** for global client settings.

**Source**: `lib.rs`, `state.rs`, `sdk/`. **Tables**: [reference.md](reference.md).

---

## Architecture

```
RakuraiActivationConfigAccount (singleton)
  authority, client_authority, commission account/bps
        │
        ▼ read at RAA init
RakuraiActivationAccount per validator_identity
  is_enabled, proposer, commissions, hash: Option<[u8;64]>
```

**Async 2-party multisig** (separate txs):

| Action | Approval |
|--------|----------|
| Enable | 2/2 — proposer set, then other party accepts |
| Disable | 1/2 — either party revokes |
| Re-enable | 2/2 again |

Client must pass `hash: Some([u8;64])` on final enable accept.

**Commission**: `block_reward_commission_bps` independent of Solana vote commission. Updates apply current epoch if no RCA yet, else next epoch (client convention).

---

## Key Accounts & Instructions

| Account | Role |
|---------|------|
| `RakuraiActivationConfigAccount` | Singleton; client authority, commission |
| `RakuraiActivationAccount` | Per-validator; scheduler, commissions, proposer, hash |

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config` |
| RAA | `initialize_rakurai_activation_account`, `update_rakurai_activation_approval`, `update_rakurai_activation_commission`, `close_rakurai_activation_account` |

---

## Activation Flow

1. **Config** (once): `initialize` — admin CLI `init-config`.
2. **RAA init**: identity signs (must match vote node pubkey); `is_enabled = false`; copies client bps from config.
3. **Enable** (2 txs): party A `update_rakurai_activation_approval(true)` → proposer; party B same ix → `is_enabled = true`.
4. **Disable** (1 tx): either party `grant_approval: false` — clears enabled/proposer/hash.
5. **Commission**: `update_rakurai_activation_commission` — validator updates `block_reward_commission_bps`; client updates `client_commission_bps`.

---

## SDK

```rust
use rakurai_activation::sdk::{
    derive_config_account_address,
    derive_activation_account_address,
};
```

Ix builders: `initialize_ix`, `update_config_ix`, `initialize_rakurai_activation_account_ix`, `update_rakurai_activation_approval_ix`, `update_rakurai_activation_commission_ix`, `close_rakurai_activation_account_ix`.

---

## Build & Program IDs

```bash
anchor build -p rakurai_activation
anchor test -p rakurai_activation
```

| Cluster | Program ID |
|---------|------------|
| mainnet | `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi` |
| testnet | `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt` |
| localnet | `CuvTfdaxcDbvvtACkXrW2j69YeQbWuNnwg9FefYDrSug` |

`declare_id!` = testnet. IDL: `programs/rakurai_activation/idl/rakurai_activation.json`.

---

## CLI

Binary: `rakurai-activation`. See [rakurai-cli skill](../rakurai-cli/SKILL.md).

| Subcommand | On-chain ix |
|------------|-------------|
| `init` | `initialize_rakurai_activation_account` |
| `scheduler-control` | `update_rakurai_activation_approval` |
| `update-commission` | `update_rakurai_activation_commission` |
| `show` | read RAA |
| `init-config` / `show-config` / `close` | admin (hidden) |

**Related**: `reward_distribution` (RCA reads RAA commissions), `rakurai_tip_manager` (separate tip path).
