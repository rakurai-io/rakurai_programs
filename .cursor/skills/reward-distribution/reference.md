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

---

## Signers (revenue share)

| Instruction | Signer | Notes |
|-------------|--------|-------|
| initialize_revenue_share_account | any payer | legacy; RD config + RAA |
| initialize_revenue_share_account_v1 | any payer | V1; tips/mev config + RAA |
| record_revenue / record_revenue_v1 | record_authority | |
| claim_revenue / claim_revenue_v1 | manager_authority | V1: Rakurai name → commission 0 |
| settle_revenue | any payer | V1 only; non-Rakurai |
| update_deficit | manager_authority | V1 only |
| close_revenue_share_account / `_v1` | manager_authority | |

---

## SDK derives

- `derive_revenue_share_account_address` / `derive_tip_collection_account_address` → legacy
- `derive_revenue_share_account_v1_address` / `derive_tip_collection_account_v1_address` → V1
