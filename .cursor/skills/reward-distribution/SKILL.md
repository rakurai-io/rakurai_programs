---
name: reward-distribution
description: >-
  Rakurai Reward Distribution program (RCA, Merkle claims, revenue-share
  tip/mev-share PDAs). Use for reward distribution, RCA, revenue-share vaults,
  Merkle claims, client commission, or reward-distribution CLI work.
---

# Rakurai Reward Distribution

`programs/reward_distribution/` (v0.3.0). Per-epoch **RCA** vault; staker claims via Merkle. Off-path tip/mev-share revenue shares tracked in per-validator PDAs (no on-chain registry).

**Source**: `lib.rs`, `state.rs`, `sdk/`. **Details**: [reference.md](reference.md).

---

## Architecture

```
RAA commission ──► RCA (vote, epoch) ──► Merkle staker claims
TipsAndMevShareConfigAccount ── defaults for Tip / MevShare at init_v1
RevenueShareAccount (share_kind = Tip | MevShare) ── per (kind, revenue label, vote)
  record_revenue (accounting) ──► claim_revenue splits commission_bps
    (0 when name == RAKURAI_REVENUE_NAME: commission already taken on tip drain)
    → commission_account + validator identity
```

- **RCA** — lamport vault, optional Merkle root; block rewards via `transfer_staker_rewards`
- **TipsAndMevShareConfigAccount** — singleton defaults (manager, commission, epoch capacity) for Tip and MevShare; used by `initialize_revenue_share_account_v1` (`record_authority` is an ix arg)
- **Revenue-share PDAs** — one unified `RevenueShareAccount` per `(share_kind, name, vote)`; epoch ledger + lamport vault; `record_revenue` is accounting-only; claim applies `commission_bps` except Rakurai-named vaults (tip drain already took that cut)

---

## Key Types

| Account | Role |
|---------|------|
| `RewardDistributionConfigAccount` | Admin, `num_epochs_valid` (1–10), commission caps, MEV toggle |
| `TipsAndMevShareConfigAccount` | Singleton tip + mev-share defaults for `initialize_revenue_share_account_v1` |
| `RewardCollectionAccount` | Per-epoch collection vault |
| `ClaimStatus` | Per-claimant replay guard |
| `RevenueShareAccount` | Unified revenue-share vault per `(share_kind, name, validator)`; `share_kind ∈ {Tip, MevShare}` in PDA seeds. Type aliases `TipsCollectionAccount` (TCA) / `MevShareCollectionAccount` (MCA) |

`RAKURAI_REVENUE_NAME` — lowercase `rakurai` padded to 32 bytes; defined in RD `state.rs`; tip-manager re-exports it.

---

## Instructions

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config`, `close_config` |
| Tips/Mev config | `initialize_tips_and_mev_share_config`, `update_tips_and_mev_share_config`, `close_tips_and_mev_share_config` |
| RCA | `initialize_reward_collection_account_v1`, `upload_merkle_root`, `transfer_staker_rewards`, `transfer_client_commission_on_mev_commission`, `close_reward_collection_account` |
| Claims | `claim`, `close_claim_status` |
| Revenue share | `initialize_revenue_share_account_v1`, `record_revenue`, `settle_revenue`, `update_transferred_amount`, `claim_revenue`, `update_deficit`, `update_revenue_share_config`, `update_epoch_converted_to_block_reward`, `close_revenue_share_account`, `close_revenue_share_account_legacy` |

---

## Epoch Flow

1. **Init RCA** (epoch E): validator identity signs; pass `validator_vote_account` + enabled RAA via `initialize_reward_collection_account_v1`.
2. **During E**: `transfer_staker_rewards` (pass same vote account; must match RCA); `record_revenue`; optional MEV commission ix.
3. **After E**: `upload_merkle_root`; staker `claim`; `claim_revenue` — commission to `commission_account` except Rakurai-named vaults (0; already taken at tip drain), remainder to validator identity.
4. **Cleanup**: close RCA after expiry; `close_claim_status` permissionless after expiry.

**Revenue share flow:** tips/mev config init → `initialize_revenue_share_account_v1` → `record_revenue` (Rakurai tip TCA also credits `transferred_amount`) → non-Rakurai `settle_revenue` or `update_transferred_amount` (direct deposit sync) → `claim_revenue`. Record: `record_authority`. Claim/config/deficit: `manager_authority`.

---

## SDK

```rust
use reward_distribution::sdk::{
    derive_config_account_address,
    derive_tips_and_mev_share_config_address,
    derive_reward_collection_account_address,
    derive_revenue_share_account_address,           // (share_kind, name, vote)
    derive_tip_collection_account_address,          // wrapper: Tip
    derive_mev_share_collection_account_address,    // wrapper: MevShare
};
```

ClaimStatus PDA: `[ClaimStatus::SEED, claimant, rca]` — no SDK helper yet.

---

## Build & Program IDs

```bash
anchor build -p reward_distribution --no-idl
```

| Cluster | Program ID |
|---------|------------|
| testnet | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` |
| mainnet | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` |
| localnet | `CtVB7ze4Kz2iUHGrWLWY9EG5Au1erbRCMvKTWFuKv8wq` |

`declare_id!` = testnet. Prefer Rust SDK over JSON IDL for new types.

**Related**: `rakurai_activation` (RAA), `rakurai_tip_manager` (tip PDAs; CPI `record_revenue` on drain).
