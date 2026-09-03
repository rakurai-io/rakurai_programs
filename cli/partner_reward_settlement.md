# Partner Tip and MevShare Revenue Settlement CLI (`rakurai-revshare`)

**Audience:** Transaction-landing services and post-pack partners.

Install: [CLI overview](./README.md#2-installation). Prepaid P2C subscription (Users/Consumers) is a different CLI: [`rakurai-p2c`](./p2c_subscription.md). Program details: [Reward Distribution](../programs/reward_distribution/README.md).

---

## 1. What this CLI is about

Partners can keep custody of collected SOL in their own accounts (custom tips and/or post-pack MevShare). Rakurai **cannot drain** those accounts, so after each epoch you must settle the agreed share into the validator’s on-chain vault.

This CLI is how you do that:

- **Read** TCA / MCA vaults (`get-account`, `get-all-accounts`, pending-record commands)
- **Record** MCA MevShare for the current epoch (`record-revenue`)
- **Settle** by transferring SOL into the vault (`transfer`, `transfer-all`)

Claim/distribution of settled funds is the **manager** flow, not this CLI.

Do **not** use this CLI for Rakurai’s own eight tip PDAs — those are drained by Tip Manager. This CLI is only for **partner-owned** custom-tip TCAs and MevShare MCAs.

`--revenue-name` is the service id Rakurai assigned when you enabled custom tips and/or post-pack. It is a PDA seed — use the exact value.

### When to use `--revenue-kind Tip` vs `Mev-share`

Pass `--revenue-kind` on every command. The two kinds are **different vaults** (different PDAs). Using the wrong kind looks at the wrong account.

| | `--revenue-kind Tip` (TCA) | `--revenue-kind Mev-share` (MCA) |
| --- | --- | --- |
| Use when | You run a **custom tip account**. Tips land in your account; the validator records the owed share each leader turn. | You share **post-pack / backrun (MevShare)** revenue. Nothing is recorded during the epoch. |
| After the epoch | **Transfer only** — the ledger is already written. | **Record, then transfer** — you write the ledger (`record-revenue`), then send SOL. |
| Who records | Validator (automatic) | You, with the MCA `record_authority` keypair |
| Command | `transfer` / `transfer-all` | `record-revenue` then `transfer` / `transfer-all` |

If a service does not record and settle within **2 epochs**, post-pack access and MCA prioritization stop after that grace period.

---

## 2. What you do each epoch

### Custom tips — `--revenue-kind Tip`

1. Wait for the epoch to end.
2. `get-account` / `get-all-accounts` — check balance, total owed, deficit (if any).
3. If epochs are pending, settle: `transfer` (one validator) or `transfer-all` (all validators for this service).
4. Keep the transaction signature.

### Post-pack MevShare — `--revenue-kind Mev-share`

1. Wait for the epoch to end.
2. `record-revenue` once per validator (MCA `record_authority` keypair; amount is the current cluster epoch).
3. `get-account` / `get-all-accounts` — confirm pending amount.
4. Settle: `transfer` or `transfer-all`.
5. Keep the transaction signatures.

---

## 3. Commands

Global flags: `-u` / `--url`, `-p` / `--program-id`, `-k` / `--keypair`.

### 3.1. `get-account`

One vault for one validator. Summary: pubkey, name, vote, balance after rent-exempt, total owed, pending count, and **deficit only if greater than zero**. `--detail` adds record authority and per-epoch rows (sorted by epoch).

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-account \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Use `--revenue-kind Mev-share` for an MCA.

### 3.2. `get-all-accounts`

All vaults for `--revenue-kind` + `--revenue-name` (no `--vote-pubkey`). Default: vote × epoch pending table. `--detail` adds per-account pubkey, record authority, balance, and pending by epoch.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-all-accounts \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME>
```

### 3.3. `get-pending-record`

One epoch on one vault. `pending = recorded − transferred`.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-pending-record \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --epoch <EPOCH>
```

### 3.4. `get-all-pending-records`

Every unsettled epoch on one vault, ordered by epoch.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-all-pending-records \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

### 3.5. `record-revenue` (MCA only)

Requires `--revenue-kind Mev-share` and the MCA `record_authority` (`Record auth` on `get-account --detail`). Updates the ledger only — **no SOL moves**. No `--epoch` flag; the program uses the current cluster epoch. Repeated calls in the same epoch accumulate.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <RECORD_AUTHORITY_KEYPAIR> \
  record-revenue \
  --revenue-kind Mev-share \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --amount <LAMPORTS>
```

See [post-epoch record and settle](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations#4.4.-post-epoch-stage-record-and-settle).

### 3.6. `transfer`

Settle one epoch on one vault. Omit `--amount` to send the full pending amount. `--dry-run` previews without sending.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <PARTNER_PAYER_KEYPAIR> \
  transfer \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --epoch <EPOCH>
```

### 3.7. `transfer-all`

Settle every pending epoch on every matching vault. Same pending table as `get-all-accounts`, then batches of **10** instructions per transaction (`--batch-size` to override). For MCA, run `record-revenue` first.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <PARTNER_PAYER_KEYPAIR> \
  transfer-all \
  --revenue-kind Tip \
  --revenue-name <REVENUE_NAME> \
  --dry-run
```

---

## Appendix: Program IDs

| Cluster | Reward Distribution program |
| ------- | --------------------------- |
| Mainnet | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` |
| Testnet | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` |

Verify RPC cluster, program ID, vote pubkey, revenue name, epoch, and lamports before recording or transferring.
