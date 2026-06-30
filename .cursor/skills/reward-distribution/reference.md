# Reward Distribution Reference

Source: `programs/reward_distribution/src/` (v0.3.0).

---

## PDA Seeds

| Account | Seeds |
|---------|-------|
| Config | `RD_CONFIG_ACCOUNT` |
| RCA | `REWARD_COLLECTION_ACCOUNT`, vote, `epoch.to_le_bytes()` |
| ClaimStatus | `CLAIM_STATUS`, claimant, rca |
| PartnerShare (Tip/Backrun) | `PARTNER_SHARE`, `share_kind_seed` (`TIP` \| `BACKRUN`), name[32], vote |

Unified account `PartnerShareAccount` (aliases `PartnerTipShareAccount` / `PartnerBackrunShareAccount`). `share_kind` selects the seed segment, so the same `(name, vote)` yields distinct Tip vs Backrun PDAs.

Space: `PartnerShareAccount::space_for(max_epoch_entries)`. `max_epoch_entries` is ledger capacity only (not in PDA seeds). Cap: `MAX_PARTNER_SHARE_EPOCH_ENTRIES_CAP = 32`.

---

## Partner Share Accounts

| Field | Type | Notes |
|-------|------|-------|
| share_kind | PartnerShareKind | `Tip` or `Backrun`; in PDA seeds |
| name | [u8; 32] | Partner/wallet label in seeds; non-empty |
| validator_vote | Pubkey | Must match RCA vote on claim |
| manager_authority | Pubkey | Set from `config.tip_backrun_manager_authority` at init; claim, update commission, update convert flag, close |
| record_authority | Pubkey | Signs `record_partner_share`; may also call `update_partner_share_convert_to_block_rewards` |
| max_epoch_entries | u8 | Ledger capacity (1–32); affects account size, not PDA seeds |
| commission_bps | u16 | Share of epoch ledger amount sent to `commission_account` on claim |
| commission_account | Pubkey | Receives commission portion; required non-default when `commission_bps > 0` |
| convert_to_block_rewards | bool | Current routing intent; snapshotted per epoch into the ledger on `record_partner_share` |
| ledger | PartnerShareLedger | `Vec<EpochAmountEntry>` |
| bump | u8 | |

**EpochAmountEntry**: `epoch`, `amount`, `claimed`, `converted_to_block_reward` (snapshot of `convert_to_block_rewards` when the epoch was first recorded). Ledger grows until capacity, then overwrites oldest entry.

**Claim split**: `commission_amount = amount * commission_bps / 10000`; remainder → RCA `initializer` (validator identity).

---

## RCA (selected fields)

| Field | Notes |
|-------|-------|
| validator_vote_account | Vote pubkey |
| creation_epoch | Partner share claim `epoch` arg |
| initializer | Validator identity; receives partner share remainder after commission |
| expires_at | Claims deadline |
| merkle_root | Optional Merkle metadata |
| block_reward_commission_bps / block_builder_commission_* | Block reward path only |

---

## Signers & vote auth

Vote binding (where applicable): vote account owned by vote program; `VoteState` node pubkey == signer (validator identity).

| Instruction | Signer | Vote account | Extra auth |
|-------------|--------|--------------|------------|
| initialize_reward_collection_account | validator identity | required | Enabled RAA PDA; vote node == signer |
| upload_merkle_root | merkle_root_upload_authority | — | — |
| claim_partner_share | manager_authority | — | — |
| update_partner_share_commission | manager_authority | — | — |
| update_partner_share_convert_to_block_rewards | manager_authority **or** record_authority | — | — |
| transfer_staker_rewards | initializer | required | vote == `RCA.validator_vote_account`; vote node == signer |
| transfer_block_builder_commission_on_mev_commission | initializer | — | — |
| initialize_partner_share_account | `config.tip_backrun_manager_authority` (payer) | — | passes `share_kind`, `commission_bps`, `commission_account` |
| record_partner_share | record_authority | — | — |
| close_partner_share_account | manager_authority | — | — |
| claim | payer | — | — |
| close_claim_status | — | — | permissionless after expiry |

---

## Instruction Accounts (RCA)

**initialize_reward_collection_account**: config, reward_collection_account (init), rakurai_activation_account, validator_vote_account, signer, system_program.

**transfer_staker_rewards**: validator_vote_account, block_builder_commission_account (mut), reward_collection_account (mut), system_program, signer (mut).

---

## Merkle Leaf

```rust
hashv(&[&[0u8], &hashv(&[claimant.as_ref(), &amount.to_le_bytes()]).to_bytes()])
```

Proof siblings use `[1u8]` prefix (`merkle_proof.rs`).

---

## Partner Share Errors

| Code | When |
|------|------|
| InvalidPartnerName | Empty partner label |
| InvalidPartnerShareEpochCapacity | `max_epoch_entries` ∉ 1..=32 |
| EpochEntryNotFound | No ledger entry for epoch |
| EpochAlreadyClaimed | Entry already claimed |
| PrematurePartnerShareClaim | `current_epoch <= epoch` |
| RewardsTooLow | Zero amount or vault under-funded |
| MaxCommissionFeeBpsExceeded | `commission_bps > config.max_commission_bps` |
| TipBackrunManagerNotConfigured | `tip_backrun_manager_authority` is `None` on config |
| RakuraiSchedulerNotEnabled | RAA exists but `is_enabled == false` (RCA init) |
| Unauthorized | Signer / vote / RAA mismatch |

---

## Config (selected fields)

| Field | Notes |
|-------|-------|
| tip_backrun_manager_authority | `Option<Pubkey>`; if `None`, partner share init is disabled |
| max_commission_bps | Caps partner share `commission_bps` at init and update |

---

## Claim Accounts

`claim`: reward_collection_account, claim_status (init), claimant, payer, system_program.

Partner share claim (`claim_partner_share`): `reward_collection_account`, `partner_share_account`, `commission_account` (must match stored pubkey), `validator_identity` (= RCA initializer), `manager_authority`.

### Partner share init accounts

`config` (RD config PDA), `partner_share_account` (init, PDA seeded with `share_kind`), `validator_vote_account`, `payer` (= `tip_backrun_manager_authority`), `system_program`. Args include `share_kind`, `commission_bps`, `commission_account`.

### Partner share commission update accounts

`update_partner_share_commission`: `partner_share_account` (mut), `config`, `manager_authority`.

`update_partner_share_convert_to_block_rewards`: `partner_share_account` (mut), `config`, `signer` (= manager_authority **or** record_authority).
