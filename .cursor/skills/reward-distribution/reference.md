# Reward Distribution Reference

Source: `programs/reward_distribution/src/` (v0.3.0).

---

## PDA Seeds

| Account | Seeds |
|---------|-------|
| Config | `RD_CONFIG_ACCOUNT` |
| TipsAndMevShareConfig | `TIPS_AND_MEV_SHARE_CONFIG` |
| RCA | `REWARD_COLLECTION_ACCOUNT`, vote, `epoch.to_le_bytes()` |
| ClaimStatus | `CLAIM_STATUS`, claimant, rca |
| Revenue Share (Tip/MevShare) | `REVENUE_SHARE`, `share_kind_seed` (`TIP` \| `MEV_SHARE`), name[32], vote |

Unified account `RevenueShareAccount` (aliases `TipsCollectionAccount` (TCA) / `MevShareCollectionAccount` (MCA)). `share_kind` selects the seed segment, so the same `(name, vote)` yields distinct Tip vs MevShare PDAs.

Space: `RevenueShareAccount::space_for(max_epoch_entries)`. `max_epoch_entries` is ledger capacity only (not in PDA seeds). Cap: `MAX_REVENUE_EPOCH_ENTRIES_CAP = 32`.

`RAKURAI_REVENUE_NAME`: lowercase `rakurai` padded to `[u8; 32]` (`state.rs`). Tip-manager re-exports the same constant.

---

## TipsAndMevShareConfigAccount

Singleton defaults copied onto TCA/MCA at `initialize_revenue_share_account_v1`.

| Field | Notes |
|-------|-------|
| authority | Updates / closes this config |
| tip_manager_authority | → TCA `manager_authority` |
| tip_commission_account / tip_commission_bps | → TCA commission fields |
| tip_epoch | → TCA `max_epoch_entries` (1..=32) |
| tip_record_authority | → TCA `record_authority` |
| mev_share_* | Same for MevShare / MCA |
| bump | PDA bump |

Validate: each side’s `commission_bps ≤ 10_000`; non-default commission account when bps > 0; epoch in 1..=32; manager and record authorities non-default.

---

## Revenue Share Accounts

| Field | Type | Notes |
|-------|------|-------|
| share_kind | RevenueKind | `Tip` or `MevShare`; in PDA seeds |
| name | [u8; 32] | Revenue source / wallet label in seeds; non-empty |
| validator_vote | Pubkey | Validator vote this vault is tied to |
| initializer | Pubkey | Payer at init; receives account rent on close |
| manager_authority | Pubkey | From tips/mev config at init; claim, update config |
| record_authority | Pubkey | Signs `record_revenue` |
| max_epoch_entries | u8 | Ledger capacity (1–32); affects account size, not PDA seeds |
| commission_bps | u16 | Claim commission to `commission_account`; forced 0 when `name == RAKURAI_REVENUE_NAME` (already taken on tip drain for Rakurai TCA) |
| commission_account | Pubkey | Receives commission portion; required non-default when `commission_bps > 0` |
| block_reward_conversion_enabled | bool | When true, epoch entries require explicit conversion via `update_epoch_converted_to_block_reward` |
| ledger | RevenueLedger | `Vec<EpochAmountEntry>` |
| bump | u8 | |

**EpochAmountEntry**: `epoch`, `amount`, `claimed`, `block_reward_converted`. Ledger grows until capacity, then overwrites oldest claimed entry.

**Claim split**: `commission_amount = amount * commission_bps / 10000`; remainder → validator identity.

**Rakurai name exception:** if `name == RAKURAI_REVENUE_NAME`, effective `commission_bps = 0` at claim. Tip-manager `change_tip_receiver_v2` already deducted Rakurai’s share on drain; re-applying at claim would double-charge. Partner TCA/MCA vaults still take commission at claim.

---

## RCA (selected fields)

| Field | Notes |
|-------|-------|
| validator_vote_account | Vote pubkey |
| creation_epoch | Epoch when RCA was initialized |
| initializer | Validator identity |
| expires_at | Claims deadline |
| merkle_root | Optional Merkle metadata |
| block_reward_commission_bps / client_commission_* | Block reward path only |

---

## Signers & vote auth

Vote binding (where applicable): vote account owned by vote program; `VoteState` node pubkey == signer (validator identity).

| Instruction | Signer | Vote account | Extra auth |
|-------------|--------|--------------|------------|
| initialize_reward_collection_account_v1 | validator identity | required | Enabled RAA PDA; vote node == signer |
| upload_merkle_root | merkle_root_upload_authority | — | — |
| claim_revenue | manager_authority | — | Rakurai name → commission 0 (avoid double fee after tip drain) |
| update_revenue_share_config | manager_authority | — | — |
| update_epoch_converted_to_block_reward | manager_authority **or** record_authority **or** validator identity (vote node) | required for validator path | entry claimed; entry flag false |
| transfer_staker_rewards | initializer | required | vote == `RCA.validator_vote_account`; vote node == signer |
| transfer_client_commission_on_mev_commission | initializer | — | — |
| initialize_tips_and_mev_share_config | any payer | — | one-time PDA init |
| update_tips_and_mev_share_config | tips/mev config authority | — | — |
| close_tips_and_mev_share_config | tips/mev config authority | — | — |
| initialize_revenue_share_account_v1 | any payer | required | enabled RAA; vote node == RAA `validator_authority`; defaults from tips/mev config |
| record_revenue | record_authority | — | — |
| close_revenue_share_account | manager_authority | — | rent to `initializer` |
| claim | payer | — | — |
| close_claim_status | — | — | permissionless after expiry |

---

## Instruction Accounts (RCA)

**initialize_reward_collection_account_v1**: config, reward_collection_account (init), rakurai_activation_account, validator_vote_account, signer, system_program.

**transfer_staker_rewards**: validator_vote_account, client_commission_account (mut), reward_collection_account (mut), system_program, signer (mut).

---

## Merkle Leaf

```rust
hashv(&[&[0u8], &hashv(&[claimant.as_ref(), &amount.to_le_bytes()]).to_bytes()])
```

Proof siblings use `[1u8]` prefix (`merkle_proof.rs`).

---

## Revenue Share Errors

| Code | When |
|------|------|
| InvalidRevenueName | Empty revenue label |
| InvalidRevenueEpochCapacity | `max_epoch_entries` ∉ 1..=32 |
| EpochEntryNotFound | No ledger entry for epoch |
| EpochAlreadyClaimed | Entry already claimed |
| EpochNotClaimed | Entry not claimed yet (`update_epoch_converted_to_block_reward`) |
| EpochAlreadyConvertedToBlockReward | Entry already `block_reward_converted` |
| PrematureRevenueClaim | `current_epoch <= epoch` |
| RewardsTooLow | Zero amount or vault under-funded |
| MaxCommissionFeeBpsExceeded | `commission_bps >` allowed max |
| RakuraiSchedulerNotEnabled | RAA exists but `is_enabled == false` |
| Unauthorized | Signer / vote / RAA mismatch |

---

## Config (selected fields)

| Field | Notes |
|-------|-------|
| max_commission_bps | Caps revenue-share `commission_bps` on `update_revenue_share_config` |
| num_epochs_valid | RCA validity window (1–10) |

### `update_config` accounts

`config` (mut, realloc to current `SIZE`), `authority` (mut, signer, pays extra rent if growing), `system_program`.

---

## Claim Accounts

`claim`: reward_collection_account, claim_status (init), claimant, payer, system_program.

Revenue claim (`claim_revenue`): `revenue_share_account`, `commission_account` (must match stored pubkey), `validator_identity`, `manager_authority`.

### Tips/mev config accounts

`initialize_tips_and_mev_share_config`: `tips_and_mev_share_config` (init), `system_program`, `initializer`.

`update_tips_and_mev_share_config`: `tips_and_mev_share_config` (mut), `authority`.

`close_tips_and_mev_share_config`: `tips_and_mev_share_config` (mut, close), `signer`.

### Revenue share init accounts

`initialize_revenue_share_account_v1`: `tips_and_mev_share_config`, `revenue_share_account` (init), `rakurai_activation_account`, `validator_vote_account`, `payer`, `system_program`. Args: `share_kind`, `name`, `bump` only (defaults from config by `share_kind`).

### Revenue share close accounts

`close_revenue_share_account`: `revenue_share_account` (mut, close), `initializer` (mut, receives rent), `authority` (signer = `manager_authority`).

### Revenue share config update accounts

`update_revenue_share_config`: `revenue_share_account` (mut), `config`, `manager_authority`.

`update_epoch_converted_to_block_reward`: `revenue_share_account` (mut), `validator_vote_account`, `signer`.
