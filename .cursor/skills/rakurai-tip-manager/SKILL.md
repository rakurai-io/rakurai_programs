---
name: rakurai-tip-manager
description: >-
  Rakurai Tip Manager on-chain program (8 tip PDAs, config singleton), tip draining,
  and commission split. Use for tip manager, change_tip_receiver, tip
  accounts, block builder commission on tips, or integrating tips with RCA/RTCA.
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

change_tip_receiver: drain → old_tip_receiver + commission; set new receiver (Rakurai validator)
change_block_builder: drain → validator receiver + old builder; update builder (authority)
```

No reward_distribution CPI. Routing tips to RCA is done off-chain (lamport transfer to current-epoch RCA); RTCA = `balance - rent - BRCA`.

---

## Instructions

| Instruction | Signer | Purpose |
|-------------|--------|---------|
| `initialize_rakurai_tip_manager` | payer | Config + 8 tip PDAs |
| `close_rakurai_tip_manager` | authority | Close all; reclaim rent |
| `change_tip_receiver` | Rakurai validator identity | Drain → old receiver + commission; set new receiver |
| `change_block_builder` | authority | Drain → validator receiver + old builder; update builder |

`change_tip_receiver` auth: enabled RAA PDA (`signer == validator_authority`); `validator_vote_account` with vote node == signer.

---

## Tip Flow

1. Users transfer SOL to any of 8 tip PDAs ([deployed addresses](reference.md#deployed-tip-account-addresses)).
2. **Periodic drain**: `change_tip_receiver` when rotating receiver (validator leader-turn client).
3. Split: `block_builder_fee = total * bps / 10000`; remainder to validator share account.
4. **RCA credit** (optional): client transfers validator-share lamports to current-epoch RCA off-chain.

`change_tip_receiver`: `old_tip_receiver` and `block_builder_commission_account` must match config.

---

## SDK

```rust
use rakurai_tip_manager::sdk::{
    derive_rakurai_tip_manager_config_account_address,
    derive_rakurai_tip_payment_account_pdas,
};
```

Builders: `initialize_rakurai_tip_manager_ix`, `close_rakurai_tip_manager_ix`, `change_tip_receiver_ix`, `change_block_builder_ix`.

---

## Build & Program IDs

```bash
anchor build -p rakurai_tip_manager
anchor test -p rakurai_tip_manager
```

| Cluster | Program ID |
|---------|------------|
| mainnet | `rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH` |
| testnet | `4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD` |
| localnet | `6z4rnNKVzSYBxqfshk1QZFgJv17KjZoirFhpWSjqQMfu` |

`declare_id!` = testnet. IDL: `programs/rakurai_tip_manager/idl/rakurai_tip_manager.json`.

**Related**: `reward_distribution` (RCA/RTCA), `rakurai_activation` (commission alignment).
