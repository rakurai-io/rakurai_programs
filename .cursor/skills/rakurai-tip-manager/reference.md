# Rakurai Tip Manager Reference

PDA seeds, account layouts, instruction accounts. Source: `programs/rakurai_tip_manager/src/`.

---

## Tip-manager PDA seeds

| Account | Seeds |
|---------|-------|
| Config | `TIP_MANAGER_CONFIG_ACCOUNT` |
| Tip accounts | `RAKURAI_TIP_ACCOUNT_0` … `_7` |
| Record authority | `RECORD_AUTHORITY` |

SDK: `derive_rakurai_tip_manager_config_account_address`, `derive_rakurai_tip_payment_account_pdas`, `derive_rakurai_tip_collection_address` (legacy), `derive_rakurai_tip_collection_v1_address`, `derive_record_authority_address`.

## Revenue-share tip PDAs (reward_distribution)

| Path | Seeds |
|------|-------|
| Legacy TCA | `[REVENUE_SHARE, TIP, RAKURAI_REVENUE_NAME, vote]` |
| TCAV1 | `[REVENUE_SHARE_V1, TIP, RAKURAI_REVENUE_NAME, vote]` |

---

## Commission handoff (v1 / v2)

Each leader turn:

1. **Drain** — split tips using tip-manager global `client_commission_bps` / `client_commission_account` (written by the **previous** leader from their TCA).
2. **Rotate** — set `validator_tip_receiver_account` to the new TCA / TCAV1.
3. **Sync** — copy **new** TCA / TCAV1 `commission_bps` / `commission_account` into global config for the **next** leader.

`client_commission_account` in the ix must match global config at drain time.

## change_tip_receiver_v1 (legacy)

`new_tip_receiver`: `TipsCollectionAccount`. Optional CPI `record_revenue`.

## change_tip_receiver_v2 (TCAV1, mirrors v1)

`new_tip_receiver`: `TipsCollectionAccountV1`. Optional CPI `record_revenue_v1`.

Auth derive: `derive_rakurai_tip_collection_v1_address`.
