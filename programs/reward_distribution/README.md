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
On the first turn of each epoch, the `RewardCollectionAccount` is automatically initialized by the Rakurai Solana client. This initialization includes:
- Commission details (from validator specific [`RakuraiActivationAccount`](../rakurai_activation/README.md))
- Authority to update the reward Merkle root (only this authority can upload merkle root to the `RewardCollectionAccount` account)
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
At the final slot of each epoch, the following process takes place:
- A snapshot of Solana accounts is captured.
- each validator’s staker details and stake weights are extracted.
- An off-chain Merkle tree is generated containing reward share data.
  - **Extra flexibility:** During this step, node operators can blacklist specific stakers or adjust individual stake weights before finalizing the tree.
- The Merkle root is uploaded to the `RewardCollectionAccount` by the `reward_merkle_root_authority`.
- Each staker claims their rewards by submitting a valid Merkle proof derived from the Merkle root. Claims are processed individually per staker.

## Reward Distribution — Free & Automated by Rakurai
- Set this authority to **Rakurai** for fully automated root generation and uploads.
- Keep it yourself if you prefer manual management.

> **0% Rakurai commission** — only standard Solana transaction fees apply.


---

## Account Lifecycle

- `RewardCollectionAccount` is valid for **10 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator’s identity account**.
  - The account is closed to reclaim rent.

---
