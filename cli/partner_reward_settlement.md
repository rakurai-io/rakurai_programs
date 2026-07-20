# Partner Tip and MevShare Revenue Settlement CLI

Inspect and settle custom tip and post-pack/MEV revenue in validator TCA/MCA vaults.

**Audience:** Transaction-landing services, searchers, traders, block engines, and post-pack / MEV partners.

**Binary:** `rakurai-partner-settle`

For install, see the [CLI overview](./README.md#2-installation). Program details: [Tip and MevShare accounts](../programs/reward_distribution/README.md#5-tip-and-mevshare-collection-accounts).

---

## 1. When to use this CLI

Use this CLI if you are a **transaction-landing service**, **searcher**, **trader**, **block engine**, or similar partner that holds tip or MEV revenue in an account **Rakurai cannot drain**, and you must settle the agreed share into the validator’s on-chain vault after the epoch.

- **TCA (Tips Collection Account)** — `--kind tip`  
  Use when you run a **custom tip account**. Tips land in your account; the validator records the owed share in the TCA each leader turn. After the epoch, transfer that amount into the TCA.

- **MCA (MevShare Collection Account)** — `--kind mev-share`  
  Use when you share **post-pack / MEV** revenue. Nothing is recorded during leader turns. After the epoch, the owed amount is recorded once in the MCA, then transfer SOL into the MCA.

Do **not** use this CLI for Rakurai’s own eight tip PDAs — those are drained by the Tip Manager. This CLI is only for **partner-owned** custom tip TCAs and MevShare MCAs.

---

## 2. Why settlement is required

Partners may register a custom tip account (or post-pack flow) so their transactions get scheduler priority while they keep custody of the collected SOL. Because Rakurai cannot drain that account, the partner must settle the agreed revenue share after the epoch:

1. Find the correct TCA or MCA (`get-account`).
2. Inspect pending epoch records (`get-pending-record` / `get-all-pending-records`).
3. Transfer the owed SOL (`transfer`).

Supports both legacy and V1 Reward Distribution account layouts.

---

## 3. Commands

### 3.1. `get-account`

Derives and fetches one partner revenue vault.

Required: `--kind`, `--name`, `--vote-pubkey`.
Default: `--account-version auto` (prefer V1, else legacy).

```sh
rakurai-partner-settle \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-account \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Output: derived address, layout (`legacy` / `v1`), type (`tip` / `mev-share`), revenue name, vote pubkey, balance.

### 3.2. `get-pending-record`

Reads the record for one epoch:

```sh
rakurai-partner-settle \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-pending-record \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --epoch <EPOCH>
```

- **V1:** `pending_amount = recorded_amount - transferred_amount`
- **Legacy:** no `transferred_amount`; `pending_amount` is the unclaimed recorded amount (not proof a prior SOL transfer did not occur)

### 3.3. `get-all-pending-records`

Lists every epoch that still needs settlement:

```sh
rakurai-partner-settle \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-all-pending-records \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Use `--kind mev-share` for MCA. Results are ordered by epoch.

### 3.4. `transfer`

Settles SOL for an existing epoch record:

```sh
rakurai-partner-settle \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <PARTNER_PAYER_KEYPAIR> \
  transfer \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --epoch <EPOCH> \
  --amount <LAMPORTS>
```

If `--amount` is omitted, transfers the full pending amount. Refuses zero, claimed, or over-pending amounts.

---

## 4. Legacy vs V1

| | Legacy | V1 |
| --- | --- | --- |
| Transfer | System transfer into the vault | `settle_revenue` (transfer + credit `transferred_amount`) |
| Retry risk | Can double-fund if you re-run without checking balance/signature | Ledger tracks what was already transferred |
| Force layout | `--account-version legacy` | `--account-version v1` |

Default `--account-version auto`: prefer V1 if it exists, else legacy; error if neither exists.

Rakurai’s own tip TCA is settled by Tip Manager — `settle_revenue` is blocked for that account. This CLI is for custom partner TCA/MCA only.

PDA derivation depends on revenue name, vote pubkey, kind, program ID, and account version. Use the exact partner revenue name agreed with Rakurai.

---

## 5. Workflows

### 5.1. Custom tips (TCA)

1. Register a custom tip account and revenue-share percentage with Rakurai — see [Tips FAQ Q4](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/rakurai_tip_manager_faqs#4.-can-i-use-my-own-tip-account-instead-of-rakurais-eight-accounts).
2. Receive tips in your account during the epoch.
3. Confirm the TCA epoch record exists.
4. Run `get-all-pending-records --kind tip` (or `get-pending-record` for one epoch).
5. After the epoch ends, run `transfer --kind tip`.
6. Keep the transaction signature for reconciliation.

### 5.2. Post-pack / MEV (MCA)

1. Complete post-pack or transaction-inclusion integration with Rakurai — see [MEV revenue sharing](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations#4.-mev-revenue-sharing).
2. Ensure the owed amount is recorded in the MCA.
3. Run `get-all-pending-records --kind mev-share` (or `get-pending-record`).
4. After the epoch ends, run `transfer --kind mev-share`.
5. Keep the transaction signature for reconciliation.

This CLI does not create revenue records or claim/distribute settled funds. Settle within the two-epoch grace period; unsettled accounts may lose prioritization.

---

## Appendix: Program IDs

| Cluster | Reward Distribution program |
| ------- | --------------------------- |
| Mainnet | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` |
| Testnet | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` |

Verify RPC cluster, program ID, vote pubkey, revenue name, epoch, and lamports before transferring.
