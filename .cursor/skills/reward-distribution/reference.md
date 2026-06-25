# Reward Distribution Reference

Source: `programs/reward_distribution/src/` (v0.3.0).

---

## PDA Seeds

| Account | Seeds |
|---------|-------|
| Config | `RD_CONFIG_ACCOUNT` |
| RCA | `REWARD_COLLECTION_ACCOUNT`, vote, `epoch.to_le_bytes()` |
| ClaimStatus | `CLAIM_STATUS`, claimant, rca |
| PartnerTipShare | `PARTNER_TIP_SHARE`, name[32], vote, `&[max_epoch_entries]` |
| PartnerBackrunShare | `PARTNER_BACKRUN_SHARE`, name[32], vote, `&[max_epoch_entries]` |

Space: `PartnerTipShareAccount::space_for(max_epoch_entries)` (same for backrun). Cap: `MAX_PARTNER_SHARE_EPOCH_ENTRIES_CAP = 32`.

---

## Partner Share Accounts

| Field | Type | Notes |
|-------|------|-------|
| name | [u8; 32] | Partner/wallet label in seeds; non-empty |
| validator_vote | Pubkey | Must match RCA vote on claim |
| manager_authority | Pubkey | Set from `config.tip_backrun_manager_authority` at init; claim, update commission, close |
| record_authority | Pubkey | Signs `record_partner_*_share` only |
| max_epoch_entries | u8 | In seeds; ledger capacity |
| commission_bps | u16 | Share of epoch ledger amount sent to `commission_account` on claim |
| commission_account | Pubkey | Receives commission portion; required non-default when `commission_bps > 0` |
| ledger | PartnerShareLedger | `Vec<EpochAmountEntry>` |
| bump | u8 | |

**EpochAmountEntry**: `epoch`, `amount`, `claimed`. Ledger grows until capacity, then overwrites oldest entry.

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
| claim_partner_*_share | manager_authority | — | — |
| update_partner_*_share_commission | manager_authority | — | — |
| transfer_staker_rewards | initializer | required | vote == `RCA.validator_vote_account`; vote node == signer |
| transfer_block_builder_commission_on_mev_commission | initializer | — | — |
| initialize_partner_*_share_account | `config.tip_backrun_manager_authority` (payer) | — | passes `commission_bps`, `commission_account` |
| record_partner_*_share | record_authority | — | — |
| close_partner_*_share_account | manager_authority | — | — |
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

Partner share claim: `reward_collection_account`, `partner_*_share_account`, `commission_account` (must match stored pubkey), `validator_identity` (= RCA initializer), `manager_authority`.

### Partner share init accounts

`config` (RD config PDA), `partner_*_share_account` (init), `validator_vote_account`, `payer` (= `tip_backrun_manager_authority`), `system_program`. Args include `commission_bps`, `commission_account`.

### Partner share commission update accounts

`partner_*_share_account` (mut), `config`, `manager_authority`.
