---
name: reward-distribution
description: >-
  Rakurai Reward Distribution program (RCA, Merkle claims, partner tip/backrun
  share PDAs). Use for reward distribution, RCA, partner share vaults, Merkle
  claims, block builder commission, or reward-distribution CLI work.
---

# Rakurai Reward Distribution

`programs/reward_distribution/` (v0.3.0). Per-epoch **RCA** vault; staker claims via Merkle. Off-path partner tip/backrun shares tracked in per-validator PDAs (no on-chain registry).

**Source**: `lib.rs`, `state.rs`, `sdk/`. **Details**: [reference.md](reference.md).

---

## Architecture

```
RAA commission ──► RCA (vote, epoch) ──► Merkle staker claims
PartnerShareAccount (share_kind = Tip | Backrun) ── per (kind, partner label, vote, max_epochs)
  record_partner_share (accounting) ──► claim_partner_share splits commission_bps to commission_account, rest to validator identity
```

- **RCA** — lamport vault, optional Merkle root; block rewards via `transfer_staker_rewards`
- **Partner share PDAs** — one unified `PartnerShareAccount` per `(share_kind, name, vote)`; epoch ledger + lamport vault; `record_partner_share` is accounting-only; claim splits per `commission_bps`

---

## Key Types

| Account | Role |
|---------|------|
| `RewardDistributionConfigAccount` | Admin, `num_epochs_valid` (1–10), commission caps, MEV toggle, `tip_backrun_manager_authority` |
| `RewardCollectionAccount` | Per-epoch collection vault |
| `ClaimStatus` | Per-claimant replay guard |
| `PartnerShareAccount` | Unified partner share vault per `(share_kind, name, validator)`; `share_kind ∈ {Tip, Backrun}` in PDA seeds. Type aliases `PartnerTipShareAccount` / `PartnerBackrunShareAccount` |

---

## Instructions

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config`, `close_config` |
| RCA | `initialize_reward_collection_account`, `upload_merkle_root`, `transfer_staker_rewards`, `transfer_block_builder_commission_on_mev_commission`, `close_reward_collection_account` |
| Claims | `claim`, `close_claim_status` |
| Partner share (unified; `share_kind` arg/stored) | `initialize_partner_share_account` (takes `share_kind`), `record_partner_share`, `claim_partner_share`, `update_partner_share_commission`, `update_partner_share_convert_to_block_rewards`, `close_partner_share_account` |

---

## Epoch Flow

1. **Init RCA** (epoch E): validator identity signs; pass `validator_vote_account` + enabled RAA; `expires_at = E + num_epochs_valid`.
2. **During E**: `transfer_staker_rewards` (pass same vote account; must match RCA); `record_partner_share`; optional MEV commission ix.
3. **After E**: `upload_merkle_root`; staker `claim`; `claim_partner_share` with `epoch = RCA.creation_epoch` and `current_epoch > epoch` — commission portion to `commission_account`, remainder to validator identity.
4. **Cleanup**: close RCA after expiry; `close_claim_status` permissionless after expiry.

Partner share init: `share_kind` (Tip|Backrun), `name[32]`, `record_authority`, `max_epoch_entries` (1–32, ledger capacity), `commission_bps`, `commission_account`, `bump`. PDA: `[PARTNER_SHARE, share_kind_seed("TIP"|"BACKRUN"), name, vote]`. Requires `config.tip_backrun_manager_authority`; manager may call `update_partner_share_commission`. The `convert_to_block_rewards` flag is settable by **manager or record authority** via `update_partner_share_convert_to_block_rewards` (also set by `update_partner_share_commission`) and snapshotted per epoch into the ledger on `record_partner_share`.

---

## SDK

```rust
use reward_distribution::sdk::{
    derive_config_account_address,
    derive_reward_collection_account_address,
    derive_partner_share_account_address,          // (share_kind, name, vote)
    derive_partner_tip_share_account_address,       // wrapper: Tip
    derive_partner_backrun_share_account_address,   // wrapper: Backrun
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
