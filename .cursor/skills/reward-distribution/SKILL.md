---
name: reward-distribution
description: >-
  Rakurai Reward Distribution program (RCA, BRCA, RTCA, Merkle claims, partner
  tip/backrun share PDAs). Use for reward distribution, RCA, partner share vaults,
  Merkle claims, block builder commission, or reward-distribution CLI work.
---

# Rakurai Reward Distribution

`programs/reward_distribution/` (v0.3.0). Per-epoch **RCA** vault; staker claims via Merkle. Off-path partner tip/backrun shares tracked in per-validator PDAs (no on-chain registry).

**Source**: `lib.rs`, `state.rs`, `sdk/`. **Details**: [reference.md](reference.md).

---

## Architecture

```
RAA commission ──► RCA (vote, epoch)
Tips ──lamports──► RCA  →  RTCA = balance - rent - BRCA
PartnerTipShareAccount / PartnerBackrunShareAccount ── per (partner label, vote, max_epochs)
```

- **RCA** — lamport vault, optional Merkle root, BRCA/RTCA (`Option` on legacy accounts)
- **BRCA** — block-reward accumulator; incremented on `transfer_staker_rewards`
- **RTCA** — derived Rakurai-tip total; refreshed after BRCA or claim changes
- **Partner share PDAs** — epoch ledger + lamport vault; `record_*` is accounting-only

---

## Key Types

| Account | Role |
|---------|------|
| `RewardDistributionConfigAccount` | Admin, `num_epochs_valid` (1–10), commission caps, MEV toggle, `tip_backrun_manager_authority` |
| `RewardCollectionAccount` | Per-epoch collection vault |
| `ClaimStatus` | Per-claimant replay guard |
| `PartnerTipShareAccount` | Partner tip share per label + validator |
| `PartnerBackrunShareAccount` | Partner backrun share per label + validator |

---

## Instructions

| Category | Instructions |
|----------|-------------|
| Config | `initialize`, `update_config`, `close_config` |
| RCA | `initialize_reward_collection_account`, `upload_merkle_root`, `transfer_staker_rewards`, `transfer_block_builder_commission_on_mev_commission`, `close_reward_collection_account` |
| Claims | `claim`, `close_claim_status` |
| Partner share | `initialize_partner_tip_share_account`, `initialize_partner_backrun_share_account`, `record_partner_tip_share`, `record_partner_backrun_share`, `claim_partner_tip_share`, `claim_partner_backrun_share`, `close_partner_tip_share_account`, `close_partner_backrun_share_account` |

---

## Epoch Flow

1. **Init RCA** (epoch E): validator identity signs; `expires_at = E + num_epochs_valid`; BRCA/RTCA = `Some(0)`.
2. **During E**: `transfer_staker_rewards`; tips to RCA (off-chain lamports); `record_partner_*_share`; optional MEV commission ix.
3. **After E**: `upload_merkle_root`; staker `claim`; partner share claim with `epoch = RCA.creation_epoch` and `current_epoch > epoch`.
4. **Cleanup**: close RCA after expiry; `close_claim_status` permissionless after expiry.

Partner share init: `name[32]` (partner/wallet label), `record_authority`, `max_epoch_entries` (1–32), `bump`. Requires `config.tip_backrun_manager_authority` set via `update_config`; signer must be that authority. `manager_authority` on the partner account is set from config.

---

## SDK

```rust
use reward_distribution::sdk::{
    derive_config_account_address,
    derive_reward_collection_account_address,
    derive_partner_tip_share_account_address,
    derive_partner_backrun_share_account_address,
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
