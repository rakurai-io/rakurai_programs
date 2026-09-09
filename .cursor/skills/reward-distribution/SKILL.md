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
RevenueShareAccount (share_kind = Tip | MevShare) ── per (kind, revenue label, vote, max_epochs)
  record_revenue (accounting) ──► claim_revenue splits commission_bps to commission_account, rest to validator identity
```

- **RCA** — lamport vault, optional Merkle root; block rewards via `transfer_staker_rewards`
- **Revenue-share PDAs** — one unified `RevenueShareAccount` per `(share_kind, name, vote)`; epoch ledger + lamport vault; `record_revenue` is accounting-only; claim splits per `commission_bps`

---

## Key Types

| Account | Role |
|---------|------|
| `RewardDistributionConfigAccount` | Admin, `num_epochs_valid` (1–10), commission caps, MEV toggle, `revenue_manager_authority` |
| `RewardCollectionAccount` | Per-epoch collection vault |
| `ClaimStatus` | Per-claimant replay guard |
| `RevenueShareAccount` | Unified revenue-share vault per `(share_kind, name, validator)`; `share_kind ∈ {Tip, MevShare}` in PDA seeds. Type aliases `TipsCollectionAccount` (TCA) / `MevShareCollectionAccount` (MCA) |

---

## Instructions

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config`, `close_config` |
| RCA | `initialize_reward_collection_account` (legacy), `initialize_reward_collection_account_v1` (preferred), `upload_merkle_root`, `transfer_staker_rewards`, `transfer_client_commission_on_mev_commission`, `close_reward_collection_account` |
| Claims | `claim`, `close_claim_status` |
| Revenue share (unified; `share_kind` arg/stored) | `initialize_revenue_share_account` (takes `share_kind`), `record_revenue`, `claim_revenue`, `update_revenue_share_config`, `update_epoch_converted_to_block_reward`, `close_revenue_share_account` |

---

## Epoch Flow

1. **Init RCA** (epoch E): validator identity signs; pass `validator_vote_account` + enabled RAA via `initialize_reward_collection_account_v1` (legacy ix omits RAA).
2. **During E**: `transfer_staker_rewards` (pass same vote account; must match RCA); `record_revenue`; optional MEV commission ix.
3. **After E**: `upload_merkle_root`; staker `claim`; `claim_revenue` with `epoch = RCA.creation_epoch` and `current_epoch > epoch` — commission portion to `commission_account`, remainder to validator identity.
4. **Cleanup**: close RCA after expiry; `close_claim_status` permissionless after expiry.

**Revenue share flow:** init → record (ledger) → settle (SOL into PDA) → claim. PDA `[REVENUE_SHARE, share_kind, name, vote]`. Init: any payer + enabled RAA; `manager_authority` from config. Record: `record_authority`. Claim/config: `manager_authority`.

---

## SDK

```rust
use reward_distribution::sdk::{
    derive_config_account_address,
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

**Related**: `rakurai_activation` (RAA), `rakurai_tip_manager` (tip PDAs; no RD CPI).
