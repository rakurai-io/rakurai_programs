---
name: rakurai-tip-manager
description: >-
  Rakurai Tip Manager on-chain program (8 tip PDAs, config singleton), tip draining,
  and commission split. Use for tip manager, change_tip_receiver_v2, tip
  accounts, client commission on tips, or integrating tips with revenue-share vaults.
---

# Rakurai Tip Manager

`programs/rakurai_tip_manager/`. Users send SOL to **8 tip PDAs** (write-lock sharding). Validators drain via **`change_tip_receiver_v2`**, or **`change_client`** (authority rotates client).

**Source**: `lib.rs`, `sdk/`. **Tables**: [reference.md](reference.md).

---

## Architecture

```
TipManagerConfigAccount (singleton): authority, receivers, bps, bumps
RakuraiTipAccount x8: empty state, lamport vaults (seeds _0.._7)

Users ──SOL──► any tip PDA

change_tip_receiver_v2: both ends TCAs; commission from old TCA; optional record_revenue CPI
change_client: drain → validator receiver + old client; update client (authority)
```

Drained validator share lands on `old_tip_receiver` (TCA). Config `validator_tip_receiver_account` rotates to `new_tip_receiver` (Rakurai TCA). When the old TCA’s `record_authority` is this program’s PDA, drain CPIs `reward_distribution::record_revenue` before lamport moves.

---

## Instructions

| Instruction | Signer | Purpose |
|-------------|--------|---------|
| `initialize_rakurai_tip_manager` | payer | Config + 8 tip PDAs |
| `close_rakurai_tip_manager` | authority | Close all; reclaim rent |
| `change_tip_receiver_v2` | Rakurai-enabled validator | TCA→TCA; commission from **old TCA**; RAA + optional `record_revenue` |
| `change_client` | authority | Drain → validator receiver + old client; update client |

Auth for drain: enabled RAA PDA; `new_tip_receiver` must be `[REVENUE_SHARE, TIP, RAKURAI_REVENUE_NAME, vote]` PDA. Remaining accounts: `[0]` RAA, `[1]` reward_distribution program. Tip-manager global `client_commission_bps` is used by `change_client`.

---

## Tip Flow

1. Init tips/mev config on RD (once) → init **TCA**: `initialize_revenue_share_account_v1` (`share_kind = Tip`, name `RAKURAI_REVENUE_NAME`).
2. Users tip any of 8 PDAs.
3. Each leader turn: `change_tip_receiver_v2` drains → old TCA + commission from old TCA; config → new TCA; may CPI `record_revenue`.
4. Settle SOL into TCA (if needed) → `claim_revenue` post-epoch. Rakurai-named TCA skips claim commission (tip drain already took it).

First drain after tip-manager init credits the init payer until config points at a TCA.

---

## SDK

```rust
use rakurai_tip_manager::sdk::{
    derive_rakurai_tip_manager_config_account_address,
    derive_rakurai_tip_payment_account_pdas,
    derive_rakurai_tip_collection_address,
    derive_record_authority_address,
};
```

Builders: `initialize_rakurai_tip_manager_ix`, `close_rakurai_tip_manager_ix`, `change_tip_receiver_v2_ix`, `change_client_ix`.

`RAKURAI_REVENUE_NAME` is re-exported from `reward_distribution::state`.

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

**Related**: `reward_distribution` (revenue-share PDAs + `record_revenue` CPI), `rakurai_activation` (RAA gate).
