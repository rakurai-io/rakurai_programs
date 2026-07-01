---
name: rakurai-tip-manager
description: >-
  Rakurai Tip Manager on-chain program (8 tip PDAs, config singleton), tip draining,
  and commission split. Use for tip manager, change_tip_receiver, tip
  accounts, block builder commission on tips, or integrating tips with revenue-share vaults.
---

# Rakurai Tip Manager

`programs/rakurai_tip_manager/`. Users send SOL to **8 tip PDAs** (write-lock sharding). Validators drain via **`change_tip_receiver`** (Rakurai-enabled validator + vote) or **`change_block_builder`** (authority rotates block builder).

**Source**: `lib.rs`, `sdk/`. **Tables**: [reference.md](reference.md).

---

## Architecture

```
TipManagerConfigAccount (singleton): authority, receivers, bps, bumps
RakuraiTipAccount x8: empty state, lamport vaults (seeds _0.._7)

Users ──SOL──► any tip PDA

change_tip_receiver: drain → old_tip_receiver + commission; config → tip receiver (legacy)
change_rakurai_tip_receiver: same drain; RAA + vote + TCA validation; config → TCA PDA
change_block_builder: drain → validator receiver + old builder; update builder (authority)
```

No reward_distribution CPI. Drained lamports land on `old_tip_receiver`; config `validator_tip_receiver_account` is set to the Rakurai **TCA** PDA for subsequent drains.

---

## Instructions

| Instruction | Signer | Purpose |
|-------------|--------|---------|
| `initialize_rakurai_tip_manager` | payer | Config + 8 tip PDAs |
| `close_rakurai_tip_manager` | authority | Close all; reclaim rent |
| `change_tip_receiver` | Rakurai validator identity | Legacy drain + rotate (no RAA/vote/TCA checks) |
| `change_rakurai_tip_receiver` | Rakurai-enabled validator | Drain + rotate; RAA enabled, vote auth, TCA PDA enforced |
| `change_block_builder` | authority | Drain → validator receiver + old builder; update builder |

`change_tip_receiver` auth: signer only (legacy). `change_rakurai_tip_receiver` auth: enabled RAA PDA; vote node == signer; `new_tip_receiver` must be `[REVENUE_SHARE, "TIP", "Rakurai", vote]` PDA (pass `reward_distribution_program`).

---

## Tip Flow

1. Init **TCA**: `initialize_revenue_share_account` (`share_kind = Tip`, name `"Rakurai"`).
2. Users tip any of 8 PDAs.
3. Each leader turn: `change_rakurai_tip_receiver` drains → `old_tip_receiver` + commission; config → TCA.
4. `record_revenue` (ledger) → settle SOL into TCA → `claim_revenue` post-epoch.

Use legacy `change_tip_receiver` only until clients migrate. First drain credits the init payer until config points at TCA.

---

## SDK

```rust
use rakurai_tip_manager::sdk::{
    derive_rakurai_tip_manager_config_account_address,
    derive_rakurai_tip_payment_account_pdas,
    derive_rakurai_tip_collection_address,
};
```

Builders: `initialize_rakurai_tip_manager_ix`, `close_rakurai_tip_manager_ix`, `change_tip_receiver_ix`, `change_rakurai_tip_receiver_ix`, `change_block_builder_ix`.

---

## Build & Program IDs

```bash
anchor build -p rakurai_tip_manager --no-idl
```

| Cluster | Program ID |
|---------|------------|
| mainnet | `rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH` |
| testnet | `4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD` |
| localnet | `6z4rnNKVzSYBxqfshk1QZFgJv17KjZoirFhpWSjqQMfu` |

**Related**: `reward_distribution` (revenue-share PDAs), `rakurai_activation` (RAA gate).
