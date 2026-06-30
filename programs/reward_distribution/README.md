# Rakurai Reward Distribution Program

A Solana smart contract for distributing block rewards among **Rakurai**, **validators**, and **stakers**. Rewards are accumulated throughout each epoch and distributed **post-epoch** to stakers using a **Merkle tree-based mechanism**.

➤ For more details, refer to the [IDL File](./idl/reward_distribution.json).

---
### Deployed Program ID
- **Mainnet**: [RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB](https://solscan.io/account/RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB)
- **Testnet**: [A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB](https://solscan.io/account/A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB?cluster=testnet)

---

## How It Works

Each **validator**, for each **epoch**, creates a unique PDA called `RewardCollectionAccount`:

- **Seeds**: `["REWARD_COLLECTION_ACCOUNT", validator_vote_pubkey, epoch_number]`
- Only the validator's **authorized withdrawer** can initialize it.
- When creating the account, the validator must specify:
  - `reward_merkle_root_authority` — Authority responsible for uploading the Merkle root post-epoch.
  - `block_reward_commission_bps` — Commission (in basis points) that the validator retains from block rewards.
  - `block_builder_commission_bps` — Commission (in basis points) for block builder (i.e: Rakurai) from block rewards.
  - `block_builder_commission_account` — Destination account for block builder commission.

> The values for `block_builder_commission_bps`, `block_reward_commission_bps`, and `block_builder_commission_account` are pulled from the [RakuraiActivationAccount](../rakurai_activation/README.md#rakuraiactivationaccount-account-creation), a validator-specific PDA (not epoch-specific), part of the [`rakurai_activation`](../rakurai_activation/README.md) program. This account controls whether the validator is running the Rakurai scheduler (and should be charged commission).

---

## 🔁 Epoch Flow

### 1. RewardCollectionAccount Account Initialization
On the first turn of each epoch, the `RewardCollectionAccount` is automatically initialized by the Rakurai Solana client. This initialization includes:
- Commission details (from validator-specific [`RakuraiActivationAccount`](../rakurai_activation/README.md)).
- Authority to update the reward Merkle root (only this authority can upload the Merkle root to the `RewardCollectionAccount` account).
> Account initialization logic is part of the Rakurai Solana client. The node operator must specify the following [CLI arguments](https://docs.rakurai.io/nodeoperator#step-5-add-additional-cli-args).


### 2. Per-Turn Transfers
During every leader turn:
- The **previous turn's block reward** is processed:
  - **Block Builder commission** → transferred to block builder (i.e: Rakurai) account.
  - **Validator commission** → remains in the validator's identity account.
  - **Staker share** → accumulated into the `RewardCollectionAccount`.

> Because the reward of the current turn is transferred during the next one, the **first turn** of an epoch handles the **last reward** of the previous epoch.

---

## 3. Post-Epoch Staker Distribution
At the final slot of each epoch, the following process takes place:
- A snapshot of Solana accounts is captured.
- Each validator's staker details and stake weights are extracted.
- An off-chain Merkle tree is generated containing reward share data.
  - **Extra flexibility**: At this stage, specific stakers can be blacklisted, and individual stake weights can be adjusted before finalizing the tree.
- The Merkle root is uploaded to the `RewardCollectionAccount` by the `reward_merkle_root_authority`.
- Each staker claims their rewards by submitting a valid Merkle proof derived from the Merkle root. Claims are processed individually per staker.


## Reward Distribution — Free & Automated by Rakurai

- Set this authority to [**Rakurai**](https://docs.rakurai.io/nodeoperator#step-5-add-additional-cli-args) for fully automated reward distribution.
- Keep it yourself if you want to do the distribution yourself.

When set to **Rakurai**, the rakurai will automatically:
1. **Create a snapshot**
2. **Calculate the Merkle root**
3. **Upload it on-chain**
4. **Distribute rewards to stakers**

<p style="font-size:14px;">
    <span style="color:#66ff66;"><i><b>0% distribution fees charged by Rakurai</b></i></span>
    — only standard Solana transaction fees apply.
</p>

---

### Block Builder Commission on MEV Rewards

Block Builder charges commission on MEV Rewards **only** if the following conditions are met:

- The validator is actively running **Rakurai during that epoch**.  
- The validator has set a non-zero **MEV commission** in their **Tip Distribution Account**.

*Note:* If the validator’s MEV commission is **0%**, Rakurai does **not** charge any commission on MEV tips.

---

### Deduction Flow

1. The validator’s share of MEV tips is credited to their **vote account** by the Tip Distribution Program in the following epoch.  
2. A `ClaimStatus` account is created to track that the validator has received MEV rewards.  
3. Rakurai client monitors the `ClaimStatus` account; once it is created, then it is eligible to deduct commission.  
4. Rakurai cannot deduct directly from the vote account, so the **same commission amount** is deducted from the validator’s **identity account** instead.  
5. The deduction is performed by invoking the `transfer_block_builder_commission_on_mev_commission` instruction in the **Reward Distribution Program**.  
6. The commission rate is defined in the **Reward Distribution Config account**.

---

## Revenue Share Accounts (Tip & Backrun)

Most revenue flows through accounts Rakurai controls directly (the eight tip PDAs, the per-epoch RCA). **Revenue share accounts exist for the cases where the revenue lands in an account Rakurai does *not* control**, so the agreed share has to be tracked and settled separately.

The unified account is `RevenueShareAccount`, parameterized by `share_kind ∈ {Tip, Backrun}`. The two kinds are exposed as type aliases: **`TipsCollectionAccount` (TCA)** and **`BackrunCollectionAccount` (BCA)**.

### Why a **Tips Collection Account** (TCA)

By default, searchers tip Rakurai's [eight tip accounts](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/rakurai_tip_manager_faqs#4.-can-i-use-my-own-tip-account-instead-of-rakurais-eight-accounts), and `rakurai_tip_manager` drains them automatically. But an external operator/searcher can [register their own custom tip account](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/rakurai_tip_manager_faqs#4.-can-i-use-my-own-tip-account-instead-of-rakurais-eight-accounts) and agree to share a commission (e.g. 30%) with Rakurai.

In that case **the tip is received in the external account holder's own account**, not in a Rakurai-controlled PDA. So Rakurai can't just drain it — instead the validator **records** the attributed amount every leader turn, and after the epoch the external account holder **settles** their agreed share into the Tips Collection Account PDA, from which the commission is deducted.

### Why a **Backrun Collection Account** (BCA)

Same idea for [post-pack confirmations](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations) used for backrun / arbitrage. The backrun revenue lands in the external account holder's own flow, so there's no account Rakurai can drain. We **trust the revenue source to share an agreed percentage** of backrun revenue; the validator records attribution per turn and the external account holder settles into the Backrun Collection Account — exactly the same flow as tips, just a different `share_kind`.

### Record → Claim flow

- **Setup**: Rakurai (the `revenue_manager_authority`) calls `initialize_revenue_share_account` once per `(share_kind, name, validator)`, fixing the label, `record_authority`, `commission_bps`, and `commission_account`.
- **Record (each leader turn)**: the `record_authority` calls `record_revenue(amount)` to accrue the attributed amount for the current epoch into the ledger. This is **accounting only** — no lamports move.
- **Settle**: after the epoch, the external account holder transfers their agreed share as plain SOL into the revenue-share PDA so it holds at least the recorded ledger amount.
- **Claim (post-epoch)**: the `manager_authority` calls `claim_revenue(epoch)` for a past epoch (`current_epoch > epoch`); the program splits `commission_bps` → `commission_account` and sends the remainder to the validator identity, marking the ledger entry `claimed`.
- **Admin**: `manager_authority` can update `commission_bps`, `commission_account`, and `convert_to_block_rewards` via `update_revenue_share_config`, or `close_revenue_share_account`; `convert_to_block_rewards` is snapshotted per epoch at first `record_revenue`.

PDA seeds: `["REVENUE_SHARE", share_kind ("TIP" | "BACKRUN"), name[32], validator_vote]`.

---

## Account Lifecycle

- `RewardCollectionAccount` is valid for **2 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator's identity account**.
  - The account is closed to reclaim rent.

---
