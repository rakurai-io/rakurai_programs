# Rakurai Reward Distribution Program

A Solana smart contract for distributing block rewards among **Rakurai**, **validators**, and **stakers**. Rewards are accumulated throughout each epoch and distributed **post-epoch** to stakers using a **Merkle tree-based mechanism**.

➤ For more details, refer to the [IDL File](./idl/reward_distribution.json).

---

## How It Works

Each **validator**, for each **epoch**, creates a unique PDA called `RewardCollectionAccount`:

- **Seeds**: `["REWARD_COLLECTION_ACCOUNT", validator_vote_pubkey, epoch_number]`
- Only the validator's **authorized withdrawer** can initialize it.
- When creating the account, the validator must specify:
  - `reward_merkle_root_authority` — Authority responsible for uploading the Merkle root post-epoch.
  - `validator_commission_bps` — Commission (in basis points) that the validator retains from block rewards.
  - `rakurai_commission_bps` — Commission (in basis points) for Rakurai from block rewards.
  - `rakurai_commission_account` — Destination account for Rakurai's commission.

> The values for `rakurai_commission_bps`, `validator_commission_bps`, and `rakurai_commission_account` are pulled from the `RakuraiActivationAccount`, a validator-specific PDA (not epoch-specific), part of the [`rakurai_activation`](../rakurai_activation/) program ([Rakurai Programs Overview](https://docs.rakurai.io/nodeoperator#2.-rakurai-smart-contracts-programs)). This account controls whether the validator is running the Rakurai scheduler (and should be charged commission).

---

## 🔁 Epoch Flow

### 1. Account Initialization
On the first turn of each epoch, the validator initializes the `RewardCollectionAccount`, providing:
- Commission details (from `RakuraiActivationAccount`)
- Authority to update the reward Merkle root
> Account initialization logic is part of rakurai solana client. The node operator must specify the following [CLI arguments](https://docs.rakurai.io/nodeoperator#step-5-add-additional-cli-args).


### 2. Per-Turn Transfers
During every leader turn:
- The **previous turn’s block reward** is processed:
  - **Rakurai commission** → transferred to Rakurai's account
  - **Validator commission** → remains in the validator's identity account
  - **Staker share** → accumulated into the `RewardCollectionAccount`

> Because the reward of the current turn is transferred during the next one, the **first turn** of an epoch handles the **last reward** of the previous epoch.

---

## 3. Post-Epoch Staker Distribution

At the end of each epoch:
1. A **snapshot** of Solana accounts is taken.
2. Staker info and stake weights are extracted for the validator.
3. A **Merkle tree** is generated with reward shares proportional to stake.
4. The **Merkle root** is uploaded to the `RewardCollectionAccount`.
5. Stakers can claim their share using a valid **Merkle proof**.

### 4. Reward Merkle Root Authority

- Set this authority to **Rakurai** if you'd like automated root generation and uploads.
- Or retain it yourself and manage the process manually.

> Rakurai charges **0% commission** for distributing rewards. Only Solana transaction fees apply.

---

## Account Lifecycle

- `RewardCollectionAccount` is valid for **10 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator’s identity account**.
  - The account is closed to reclaim rent.

---
