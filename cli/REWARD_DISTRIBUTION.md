# Reward Distribution Partner CLI

Inspect and settle custom tip and post-pack/MEV revenue in validator TCA/MCA
vaults.

**Audience:** Transaction inclusion services, block engines, MEV searchers, and
post-pack confirmation partners that share revenue with Rakurai validators.

**References:**

- [Rakurai Transaction Inclusion](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/readme)
- [Transaction Inclusion guide](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/transaction_inclusion)
- [Tips FAQ](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/rakurai_tip_manager_faqs)

---

## 1. Why is this CLI required?

Rakurai supports transaction-inclusion partners, block engines, searchers, and
post-pack confirmation consumers. Partners may register custom tip accounts so
their transactions and bundles receive Rakurai scheduler priority while the
partner retains custody of the collected SOL.

Because Rakurai cannot drain a partner-owned account, the partner must settle
the agreed revenue share after the epoch:

- **Custom tips:** the owed amount is recorded in the validator's Tips
  Collection Account (TCA). The partner transfers that amount after the epoch.
- **Post-pack / MEV revenue:** the owed amount is recorded in the validator's
  MevShare Collection Account (MCA). The partner then transfers that amount.

This CLI gives partners the small operational surface needed to find the
correct TCA/MCA, inspect an epoch's pending record, and transfer the owed SOL.
It supports both the legacy and V1 Reward Distribution account layouts.

---

## 2. Supported commands

The binary is `rakurai-reward-distribution`.

### 2.1. `get-account`

Derives and fetches one partner revenue vault.

Required target flags: `--kind`, `--name`, `--vote-pubkey`.

Default account layout: `--account-version auto` (prefer V1, fall back to
legacy).

```sh
rakurai-reward-distribution \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-account \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

The output includes the derived address, account layout (`legacy` or `v1`),
type (`tip` or `mev-share`), revenue name, validator vote pubkey, and balance.

### 2.2. `get-pending-record`

Reads the existing record for one epoch:

```sh
rakurai-reward-distribution \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-pending-record \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --epoch <EPOCH>
```

For V1 accounts, `pending_amount` is:

```text
recorded_amount - transferred_amount
```

For legacy accounts, the layout does not store `transferred_amount`.
`pending_amount` therefore means the unclaimed recorded amount, not proof that
no previous SOL transfer occurred.

### 2.3. `get-all-pending-records`

Lists every epoch that still requires settlement:

```sh
rakurai-reward-distribution \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-all-pending-records \
  --kind tip \
  --name <PARTNER_REVENUE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Pass `--kind mev-share` for MCA.

For legacy accounts, this returns every unclaimed record with a non-zero
recorded amount. For V1 accounts, it returns every unclaimed record where
`recorded_amount` is greater than `transferred_amount`. Results are ordered by
epoch.

### 2.4. `transfer`

Settles SOL for an existing epoch record:

```sh
rakurai-reward-distribution \
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

If `--amount` is omitted, the CLI transfers the full reported pending amount.
It refuses zero-value transfers, claimed epochs, and amounts greater than the
pending record.

---

## 3. Legacy and V1 behavior

### 3.1. Legacy TCA/MCA

Legacy accounts contain the recorded amount and claim status but do not track
how much SOL has been transferred. The CLI uses a normal system transfer to
fund the vault.

Before retrying a legacy settlement, verify the previous transaction signature
and vault balance. Re-running a legacy transfer can fund the same record twice
because the account has no per-epoch transfer counter.

### 3.2. V1 TCA/MCA

V1 accounts store both `amount` and `transferred_amount`. The CLI calls the
Reward Distribution `settle_revenue` instruction, which atomically:

1. transfers SOL from the partner payer to the TCA/MCA; and
2. credits the epoch's `transferred_amount`.

A plain system transfer is not sufficient for V1 partner settlement because it
would fund the vault without updating the epoch ledger.

Rakurai's own V1 tip TCA is a special case: its tip-manager flow transfers and
credits revenue automatically. `settle_revenue` is intentionally unavailable
for that account. This partner CLI settlement flow is for custom partner TCA
and MCA accounts.

---

## 4. Selecting the account version

The default is `--account-version auto`:

- prefer V1 when it exists;
- otherwise use legacy when it exists;
- error only when neither exists.

Pass `--account-version v1` or `--account-version legacy` to force one layout
with no fallback.

The revenue name, validator vote pubkey, account kind, program ID, and account
version all affect PDA derivation. Use the exact partner revenue name agreed
with Rakurai.

---

## 5. Partner settlement workflows

### 5.1. Custom tip account

1. Register the custom tip account and revenue-share percentage with Rakurai.
2. Receive tips in the partner-owned account during the epoch.
3. Confirm that the corresponding TCA epoch record exists.
4. Run `get-all-pending-records --kind tip`, or query one epoch with
   `get-pending-record --kind tip`.
5. After the epoch ends, run `transfer --kind tip`.
6. Record the transaction signature for reconciliation.

### 5.2. Post-pack / MEV share

1. Complete the post-pack or transaction-inclusion integration with Rakurai.
2. Ensure the owed revenue has been recorded in the validator's MCA.
3. Run `get-all-pending-records --kind mev-share`, or query one epoch with
   `get-pending-record --kind mev-share`.
4. After the epoch ends, run `transfer --kind mev-share`.
5. Record the transaction signature for reconciliation.

The CLI does not create revenue records or claim/distribute settled funds.
Those records must already exist, and the configured manager performs the
subsequent claim flow.

Partners should settle within the documented two-epoch grace period. Accounts
that remain unsettled may stop being used for transaction prioritization.

---

## Appendix: Program addresses

### Mainnet

- Reward Distribution program:
  `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB`

### Testnet

- Reward Distribution program:
  `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB`

Always verify the RPC cluster, Reward Distribution program ID, validator vote
pubkey, partner revenue name, epoch, and lamport amount before transferring.
