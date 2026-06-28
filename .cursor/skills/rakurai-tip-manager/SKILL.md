---
name: rakurai-tip-manager
description: >-
  Rakurai Tip Manager on-chain program (8 tip PDAs, config singleton), tip draining,
  and commission split. Use for tip manager, change_tip_receiver, tip
  accounts, block builder commission on tips, or integrating tips with partner share vaults.
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

change_tip_receiver: drain → old_tip_receiver + commission; config → partner tip-share PDA
change_block_builder: drain → validator receiver + old builder; update builder (authority)
```

No reward_distribution CPI. Drained lamports land on `old_tip_receiver`; config `validator_tip_receiver_account` is set to the Rakurai `PartnerTipShareAccount` PDA for subsequent drains.

---

## Instructions

| Instruction | Signer | Purpose |
|-------------|--------|---------|
| `initialize_rakurai_tip_manager` | payer | Config + 8 tip PDAs |
| `close_rakurai_tip_manager` | authority | Close all; reclaim rent |
| `change_tip_receiver` | Rakurai validator identity | Drain → old receiver + commission; set config receiver to partner tip-share PDA |
| `change_block_builder` | authority | Drain → validator receiver + old builder; update builder |

`change_tip_receiver` auth: enabled RAA PDA; vote node == signer. `new_tip_receiver` must be `[PARTNER_TIP_SHARE, "Rakurai", vote]` PDA on reward_distribution (pass `reward_distribution_program` account).

---

## Tip Flow

1. Rakurai inits partner vault: `initialize_partner_tip_share_account` (reward_distribution).
2. Users transfer SOL to any of 8 tip PDAs.
3. Validator calls `change_tip_receiver` on leader turns:
   - Split: `block_builder_fee = total * bps / 10000`; remainder → `old_tip_receiver`
   - Config receiver → partner tip-share PDA
4. Validator records attributed amounts: `record_partner_tip_share`.
5. Post-epoch: Rakurai `claim_partner_tip_share` (commission split).

First drain after tip-manager init credits `old_tip_receiver` (initially payer), not the partner PDA, until config already points at the partner vault.

---

## SDK

```rust
use rakurai_tip_manager::sdk::{
    derive_rakurai_tip_manager_config_account_address,
    derive_rakurai_tip_payment_account_pdas,
    derive_rakurai_partner_tip_share_address,
};
```

Builders: `initialize_rakurai_tip_manager_ix`, `close_rakurai_tip_manager_ix`, `change_tip_receiver_ix`, `change_block_builder_ix`.

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

**Related**: `reward_distribution` (partner share PDAs), `rakurai_activation` (RAA gate).
