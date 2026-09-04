# P2C Subscription CLI (`rakurai-p2c`)

**Audience:** P2C User/Consumer (searchers / traders / services that consume Rakurai post-pack confirmations).

**Program:** Reward Distribution **PSA** (`P2CSubscriptionAccount`).

Install: [CLI overview](./README.md#2-installation). How billing works on-chain: [PSA](../programs/reward_distribution/README.md#4-psa--prepaid-fee-to-use-post-pack).

---

## 1. What this CLI is about

Post-pack confirmations need a **prepaid subscription escrow** (PSA) per service name + validator vote. You keep that escrow funded; each epoch a stake-based fee is deducted from it. If the escrow runs dry and a deficit builds up, P2C access can move to grace and then stop until you clear it.

This CLI is how **you** manage that escrow:

- **Create** the PSA (`create-account`) — defaults from on-chain `P2CConfigAccount`
- **Read** balance, status, and deficit (`get-account`, `get-all-accounts`)
- **Fund** prepaid SOL (`fund`, or a plain `solana transfer` to the PDA)
- **Clear deficit** when underfunded claims left a shortfall (`clear-deficit`)

This is **not** MevShare revenue sharing. Sharing post-pack backrun profit uses an **MCA** and [`rakurai-revshare`](./partner_reward_settlement.md). Post-pack product overview: [official guide](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations).

### Post-pack setup order (required)

1. `rakurai-p2c create-account` — open the PSA for your service + validator
2. `rakurai-p2c fund` — top up prepaid SOL
3. `rakurai-revshare create-account` — open the MCA for MevShare settlement
4. **Then** start using post-pack confirmations

---

## 2. What you do each epoch

1. Keep the prepaid account funded **before** the epoch ends (`fund`, or `solana transfer` to the PDA).
2. After the epoch, run `get-account` and check:
   - balance after rent-exempt
   - status (`Active` / `InGrace` / `Suspended`)
   - deficit (shown only when greater than zero)
3. If balance is low, `fund`.
4. If a deficit exists, `clear-deficit` so status can return to `Active`.

`get-all-accounts` lists every subscription for the same `--name`. `--detail` adds per-epoch ledger rows.

### Status meanings

| Status | Meaning |
|--------|---------|
| `Active` | Fees current; P2C eligible |
| `InGrace` | Recent underfunded periods within the grace window (default **2** epochs) |
| `Suspended` | Unpaid streak exceeded grace — **P2C access is stopped** until the deficit is cleared |

---

## 3. Commands

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <KEYPAIR> \
  <COMMAND>
```

Global flags: `-k` / `--keypair`, `-u` / `--url`, `-p` / `--program-id`.

| Command | Purpose |
|---------|---------|
| `create-account` | Create PSA (defaults from `P2CConfig`; name `rakurai` blocked) |
| `get-account` | Show one escrow (balance after rent-exempt, status, deficit if any) |
| `get-all-accounts` | List escrows for a service `--name` |
| `fund` | Top up prepaid SOL |
| `clear-deficit` | Pay down open deficit (restores `Active` when fully cleared) |

### 3.1. `create-account`

Creates a PSA for `--name` + `--vote-pubkey`. Manager / record / commission / grace / ledger size come from the on-chain **P2C config** (must already exist).

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <PAYER_KEYPAIR> \
  create-account \
  --name <SERVICE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Pass `--dry-run` to preview without sending.

### 3.2. `get-account`

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-account \
  --name <SERVICE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY>
```

Add `--detail` for per-epoch ledger rows.

### 3.3. `get-all-accounts`

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  get-all-accounts \
  --name <SERVICE_NAME>
```

### 3.4. `fund`

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <FUNDER_KEYPAIR> \
  fund \
  --name <SERVICE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --amount <LAMPORTS>
```

Anyone may also fund the PDA with a plain `solana transfer` into the escrow address.

### 3.5. `clear-deficit`

Requires `--validator-identity`. Omit `--amount` to clear the full open deficit.

```sh
rakurai-p2c \
  --url <RPC_URL> \
  --program-id <REWARD_DISTRIBUTION_PROGRAM_ID> \
  --keypair <FUNDER_KEYPAIR> \
  clear-deficit \
  --name <SERVICE_NAME> \
  --vote-pubkey <VALIDATOR_VOTE_PUBKEY> \
  --validator-identity <VALIDATOR_IDENTITY>
```

Pass `--dry-run` to preview without sending a transaction.

---

## Appendix: Program IDs

| Cluster | Reward Distribution program |
| ------- | --------------------------- |
| Mainnet | `RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB` |
| Testnet | `A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB` |
