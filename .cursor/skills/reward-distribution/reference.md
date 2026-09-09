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
| Legacy revenue share | `REVENUE_SHARE`, `TIP` \| `MEV_SHARE`, name[32], vote |
| V1 revenue share | `REVENUE_SHARE_V1`, `TIP` \| `MEV_SHARE`, name[32], vote |

`RAKURAI_REVENUE_NAME`: lowercase `rakurai` padded to `[u8; 32]`.

---

## Dual vaults

| | Legacy | V1 |
|--|--------|-----|
| Type | `RevenueShareAccount` | `RevenueShareAccountV1` |
| Layout | No transferred/deficit | `transferred_amount` + `deficit` |
| Claim | Pays `amount` if vault funded | Pays `transferred_amount`; underfund → deficit |
| Record | `record_revenue` | `record_revenue_v1` (Rakurai tip auto-credits transferred) |
| Record+transfer | N/A | `record_and_transfer` (record + settle current epoch; not Rakurai tip) |
| Settle | N/A (lamports >= amount on claim) | `settle_revenue` / `update_transferred_amount` |

---

## Signers (revenue share)

| Instruction | Signer | Notes |
|-------------|--------|-------|
| initialize_revenue_share_account | any payer | legacy; RD config + RAA |
| initialize_revenue_share_account_v1 | any payer | V1; tips/mev config + RAA |
| record_revenue / record_revenue_v1 | record_authority | |
| record_and_transfer | record_authority + payer | V1 only; current epoch; non-Rakurai tip |
| claim_revenue / claim_revenue_v1 | manager_authority | V1: Rakurai name → commission 0 |
| settle_revenue | any payer | V1 only; non-Rakurai |
| update_deficit | manager_authority | V1 only |
| close_revenue_share_account / `_v1` | manager_authority | |

---

## P2C subscription escrow (post-pack confirmation prepaid)

PDA: `[P2C_SUBSCRIPTION, name[32], vote]`. Manager-only create/operations for ledger; anyone can fund. Billing for Pack-to-Chain / [post-pack confirmation](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations) Users/Consumers (not MCA MevShare).

| Instruction | Signer | Notes |
|-------------|--------|-------|
| initialize_p2c_subscription_account | manager_authority | Pays rent; stores `record_authority` for BR convert |
| fund_p2c_subscription | any funder | Or plain SOL transfer into PDA |
| record_p2c_subscription | **manager_authority** | Upload epoch stake + amount_due |
| claim_epoch_p2c_subscription | **manager_authority** | Pay `min(remaining, free)`; `force_claim` closes with deficit |
| clear_p2c_deficit | any funder | Transfer + pay commission/identity against deficit |
| update_p2c_epoch_converted_to_block_reward | manager **or** record_authority **or** vote identity | Post-claim only |
| update_p2c_subscription_config / update_p2c_deficit / close | manager_authority | Close blocked if unclaimed epochs |

`record_authority` does **not** sign epoch records — only convert-to-block (optional path).

---

## SDK derives

- `derive_revenue_share_account_address` / `derive_tip_collection_account_address` → legacy
- `derive_revenue_share_account_v1_address` / `derive_tip_collection_account_v1_address` → V1
