---
name: reward-distribution
description: >-
  Rakurai Reward Distribution program (RCA, Merkle claims, revenue-share
  tip/mev-share PDAs). Use for reward distribution, RCA, revenue-share vaults,
  Merkle claims, client commission, or Partner Tip and MevShare Revenue Settlement CLI work.
---

# Rakurai Reward Distribution

`programs/reward_distribution/` (v0.3.0). Per-epoch **RCA** vault; staker claims via Merkle. Tip/mev-share revenue in per-validator PDAs.

**Source**: `lib.rs`, `state.rs`, `sdk/`. **Details**: [reference.md](reference.md).

---

## Architecture

```
RAA commission ──► RCA (vote, epoch) ──► Merkle staker claims
TipsAndMevShareConfigAccount ── defaults for Tip / MevShare at init_v1
Legacy RevenueShareAccount ── [REVENUE_SHARE, TIP|MEV_SHARE, name, vote]
RevenueShareAccountV1 (TCAV1/MCAV1) ── [REVENUE_SHARE_V1, TIP|MEV_SHARE, name, vote]
```

- **Legacy TCA/MCA** — original layout; original ix names (`record_revenue`, `claim_revenue`, …)
- **TCAV1/MCAV1** — `transferred_amount` + `deficit`; `_v1` ixs + settle/deficit
- Old validators keep legacy; new releases use V1

---

## Key Types

| Account | Role |
|---------|------|
| `RewardDistributionConfigAccount` | Admin, `num_epochs_valid`, commission caps |
| `TipsAndMevShareConfigAccount` | Defaults for `initialize_revenue_share_account_v1` |
| `P2CConfigAccount` | Defaults for `initialize_p2c_subscription_account` |
| `RevenueShareAccount` / `TipsCollectionAccount` | Legacy vault |
| `RevenueShareAccountV1` / `TipsCollectionAccountV1` | V1 vault (deficit layout) |
| `P2CSubscriptionAccount` | PSA prepaid escrow |

`RAKURAI_REVENUE_NAME` — lowercase `rakurai` padded to 32 bytes.

---

## Instructions

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config`, `close_config` |
| Tips/Mev config | `initialize_tips_and_mev_share_config`, `update_*`, `close_*` |
| P2C config | `initialize_p2c_config`, `update_p2c_config`, `close_p2c_config` |
| RCA | `initialize_reward_collection_account_v1`, `upload_merkle_root`, `transfer_staker_rewards`, … |
| Legacy revenue | `initialize_revenue_share_account`, `record_revenue`, `claim_revenue`, `update_revenue_share_config`, `update_epoch_converted_to_block_reward`, `close_revenue_share_account` |
| V1 revenue | `initialize_revenue_share_account_v1`, `record_revenue_v1`, `record_and_transfer`, `settle_revenue`, `update_transferred_amount`, `claim_revenue_v1`, `update_deficit`, `clear_deficit_v1`, `update_revenue_share_config_v1`, `update_epoch_converted_to_block_reward_v1`, `close_revenue_share_account_v1` |
| P2C / PSA | `initialize_p2c_subscription_account` (any payer; defaults from P2C config), `fund_*`, `record_*`, `claim_*`, `clear_p2c_deficit`, … |

---

## SDK

```rust
use reward_distribution::sdk::{
    derive_revenue_share_account_address,      // REVENUE_SHARE
    derive_revenue_share_account_v1_address,   // REVENUE_SHARE_V1
    derive_tip_collection_account_address,
    derive_tip_collection_account_v1_address,
};
```

**Related**: `rakurai_activation` (RAA), `rakurai_tip_manager` (v1 = legacy TCA; v2 = TCAV1).
