# Partner Tip and MevShare Revenue Settlement CLI (`rakurai-revshare`)

**Audience:** Transaction-landing services and post-pack partners.

Install: [CLI overview](./README.md#2-installation). Prepaid P2C subscription (Users/Consumers): [`rakurai-p2c`](./p2c_subscription.md). Program details: [Reward Distribution](../programs/reward_distribution/README.md).

---

## 1. What this CLI is about

Partners can keep custody of collected SOL in their own accounts (custom tips and/or post-pack MevShare). Rakurai **cannot drain** those accounts, so after each epoch you must settle the agreed share into the validator’s on-chain vault.

This CLI is how you do that:

- **Create** a partner **MCA** (`create-account`) — TCA create is **not** offered here
- **Read** TCA / MCA vaults (`get-account`, `get-all-accounts`, pending-record commands)
- **Record** MCA MevShare for the current epoch (`record-revenue`)
- **Settle** by transferring SOL into the vault (`transfer`, `transfer-all`)

Claim/distribution of settled funds is the **manager** flow, not this CLI.

`--revenue-name` is the service id Rakurai assigned when you enable custom tips and/or post-pack. It is a PDA seed — use the exact value.

### Post-pack setup order (required)

Before using post-pack confirmations / MevShare settlement:

1. Create and fund a **PSA** with [`rakurai-p2c create-account`](./p2c_subscription.md) then `fund`
2. Create an **MCA** with `rakurai-revshare create-account` (this CLI)
3. Then start post-pack traffic and settle each epoch

### When to use `--revenue-kind Tip` vs `Mev-share`

Pass `--revenue-kind` on read / record / transfer commands. The two kinds are **different vaults**.

| | `--revenue-kind Tip` (TCA) | `--revenue-kind Mev-share` (MCA) |
| --- | --- | --- |
| Use when | You run a **custom tip account**. Tips land in your account; the validator records the owed share each leader turn. | You share **post-pack / backrun (MevShare)** revenue. Nothing is recorded during the epoch. |
| Create via this CLI | **Blocked** (Tip Manager / ops path) | `create-account` |
| After the epoch | **Transfer only** | **Record, then transfer** |
| Who records | Validator (automatic) | You, with the MCA `record_authority` keypair |

If a service does not record and settle within **2 epochs**, post-pack access and MCA prioritization stop after that grace period.

---

## 2. What you do each epoch

### Custom tips — `--revenue-kind Tip`

1. Wait for the epoch to end.
2. `get-account` / `get-all-accounts` — check balance, total owed, deficit (if any).
3. If epochs are pending, settle: `transfer` or `transfer-all`.
4. Keep the transaction signature.

### Post-pack MevShare — `--revenue-kind Mev-share`

1. Wait for the epoch to end.
2. `record-revenue` once per validator (MCA `record_authority` keypair).
3. `get-account` / `get-all-accounts` — confirm pending amount.
4. Settle: `transfer` or `transfer-all`.
5. Keep the transaction signatures.

---

## 3. Commands

Global flags: `-u` / `--url`, `-p` / `--program-id`, `-k` / `--keypair`.

### 3.1. `create-account` (MCA only)

Creates a **Mev-Share Collection Account (MCA)** for one validator. Manager / commission defaults come from on-chain TipsAndMevShare config. The validator must have an **enabled** Rakurai Activation Account (RAA).

**Blocked:** TCA (`Tip`) create, and reserved name `rakurai`.

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <PAYER_KEYPAIR> \
  create-account \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --record-authority <RECORD_AUTHORITY_PUBKEY> \
  --activation-program-id <RAKURAI_ACTIVATION_PROGRAM_ID>
```

Pass `--dry-run` to preview without sending.

### 3.2. `get-account`

One vault for one validator. Summary: pubkey, name, vote, balance after rent-exempt, total owed, pending count, and **deficit only if greater than zero**. `--detail` adds record authority and per-epoch rows (sorted by epoch).

```sh
rakurai-revshare \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-account \
  --revenue-kind Mev-share \
  --revenue-name <REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

### 3.3. `get-all-accounts`

All vaults for `--revenue-kind` + `--revenue-name`. Default: vote × epoch pending table. `--detail` adds per-account fields.

### 3.4. `get-pending-record` / `get-all-pending-records`

Inspect one epoch or every unsettled epoch on one vault.

### 3.5. `record-revenue` (MCA only)

Requires `--revenue-kind Mev-share` and the MCA `record_authority`. Ledger only — **no SOL moves**. Current cluster epoch.

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

### 3.6. `transfer` / `transfer-all`

Settle one epoch on one vault, or all pending epochs across matching vaults. `--dry-run` supported.

---

## Appendix: Program IDs

| Cluster | Reward Distribution | Rakurai Activation |
| ------- | ------------------- | ------------------ |
| Mainnet | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` | `rAKACC6Qw8HYa87ntGPRbfYEMnK2D9JVLsmZaKPpMmi` |
| Testnet | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` | `pmQHMpnpA534JmxEdwY3ADfwDBFmy5my3CeutHM2QTt` |

Verify RPC cluster, program IDs, vote pubkey, revenue name, epoch, and lamports before creating, recording, or transferring.
