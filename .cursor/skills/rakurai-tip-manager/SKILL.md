---
name: rakurai-tip-manager
description: >-
  Rakurai Tip Manager on-chain program (8 tip PDAs, config singleton), tip draining,
  and commission split. Use for tip manager, change_tip_receiver_v2, tip
  accounts, client commission on tips, or integrating tips with revenue-share vaults.
---

# Rakurai Tip Manager

`programs/rakurai_tip_manager/`. Users send SOL to **8 tip PDAs**. Validators drain via versioned tip-receiver ixs.

**Source**: `lib.rs`, `sdk/`. **Tables**: [reference.md](reference.md).

---

## Architecture

```
TipManagerConfigAccount (singleton)
RakuraiTipAccount x8

change_tip_receiver / v1  → legacy TCA (REVENUE_SHARE) + record_revenue
change_tip_receiver_v2    → TCAV1 (REVENUE_SHARE_V1) + record_revenue_v1
                          → drain: TM global commission (previous leader)
                          → after drain: sync global from new TCAV1
change_client             → drain + rotate client (authority)
```

---

## Instructions

| Instruction | Signer | Tip receiver | Commission | Record CPI |
|-------------|--------|--------------|------------|------------|
| `change_tip_receiver` | validator | unchecked | TM global bps | none |
| `change_tip_receiver_v1` | Rakurai-enabled validator | legacy TCA | TM global bps (previous leader); sync from new TCA after | `record_revenue` |
| `change_tip_receiver_v2` | Rakurai-enabled validator | TCAV1 (mirror of v1) | TM global bps (previous leader); sync from new TCAV1 after | `record_revenue_v1` |

Auth: enabled RAA + reward_distribution program in remaining accounts. `new_tip_receiver` must match derive for the path (legacy TIP or V1).

---

## Tip Flow

1. Legacy: init TCA → `change_tip_receiver_v1` (old validators unchanged).
2. V1: init TCAV1 (`initialize_revenue_share_account_v1`) → `change_tip_receiver_v2`.
3. Claim: legacy `claim_revenue` or V1 `claim_revenue_v1`.

---

## SDK

```rust
use rakurai_tip_manager::sdk::{
    derive_rakurai_tip_collection_address,      // REVENUE_SHARE
    derive_rakurai_tip_collection_v1_address,   // REVENUE_SHARE_V1
    derive_record_authority_address,
    change_tip_receiver_v1_ix,
    change_tip_receiver_v2_ix,
};
```

**Related**: `reward_distribution` (legacy + TCAV1), `rakurai_activation` (RAA gate).
