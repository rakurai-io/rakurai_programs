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
  - `client_commission_bps` — Commission (in basis points) for client (i.e: Rakurai) from block rewards.
  - `client_commission_account` — Destination account for client commission.

> The values for `client_commission_bps`, `block_reward_commission_bps`, and `client_commission_account` are pulled from the [RakuraiActivationAccount](../rakurai_activation/README.md#rakuraiactivationaccount-account-creation), a validator-specific PDA (not epoch-specific), part of the [`rakurai_activation`](../rakurai_activation/README.md) program. This account controls whether the validator is running the Rakurai scheduler (and should be charged commission).

---

## 🔁 Epoch Flow

### 1. RewardCollectionAccount Account Initialization
On the first turn of each epoch, the `RewardCollectionAccount` is automatically initialized by the Rakurai Solana client. This initialization includes:
- Commission details (from validator-specific [`RakuraiActivationAccount`](../rakurai_activation/README.md)).
- Authority to update the reward Merkle root (only this authority can upload the Merkle root to the `RewardCollectionAccount` account).
> Account initialization logic is part of the Rakurai Solana client.


### 2. Per-Turn Transfers
During every leader turn:
- The **previous turn's block reward** is processed:
  - **Client commission** → transferred to client (i.e: Rakurai) account.
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

- Set this merkle root authority to *Rakurai*(../../../validators/setup_and_build.md#4-add-additional-cli-args) for fully automated reward distribution.
- Keep it yourself if you want to do the distribution yourself.

When set to **Rakurai**, Rakurai will automatically:
1. **Create a snapshot**
2. **Calculate the Merkle root**
3. **Upload it on-chain**
4. **Distribute rewards to stakers**

<p style="font-size:14px;">
    <span style="color:#66ff66;"><i><b>0% distribution fees charged by Rakurai</b></i></span>
    — only standard Solana transaction fees apply.
</p>

---

### Client Commission on MEV Rewards

The client charges commission on MEV Rewards **only** if the following conditions are met:

- The validator is actively running **Rakurai during that epoch**.  
- The validator has set a non-zero **MEV commission** in their **Tip Distribution Account**.

*Note:* If the validator’s MEV commission is **0%**, Rakurai does **not** charge any commission on MEV tips.

---

### Deduction Flow

1. The validator’s share of MEV tips is credited to their **vote account** by the Tip Distribution Program in the following epoch.  
2. A `ClaimStatus` account is created to track that the validator has received MEV rewards.  
3. Rakurai client monitors the `ClaimStatus` account; once it is created, then it is eligible to deduct commission.  
4. Rakurai cannot deduct directly from the vote account, so the **same commission amount** is deducted from the validator’s **identity account** instead.  
5. The deduction is performed by invoking the `transfer_client_commission_on_mev_commission` instruction in the **Reward Distribution Program**.  
6. The commission rate is defined in the **Reward Distribution Config account**.

---

## Tip & MevShare Collection Accounts

Most revenue flows through accounts controlled by the [Rakurai Tip Manager Program](../rakurai_tip_manager/README.md). **Tip & MevShare Collection Accounts exist for the cases where the revenue lands in an account Rakurai does *not* control**, so the agreed share has to be tracked and settled separately.

Both are the same underlying `RevenueShareAccount`, parameterized by `share_kind ∈ {Tip, MevShare}` and exposed as the type aliases **`TipsCollectionAccount` (TCA)** and **`MevShareCollectionAccount` (MCA)**. Each account is uniquely tied to a validator, a searcher, and a share kind (Tip/MevShare). So for a single validator, you create a TCA to use a custom commission account, and an MCA to use post-pack confirmation.

### Why a **Tips Collection Account** (TCA)

By default, searchers tip Rakurai's [eight tip accounts](../rakurai_tip_manager/README.md), and `rakurai_tip_manager` drains them automatically. But an external operator/searcher can [register their own custom tip account](../../../transaction_inclusion/rakurai_tip_manager_faqs.md#4-can-i-use-my-own-tip-account-instead-of-rakurais-eight-accounts) and agree to share a commission (e.g. 30%) with Rakurai.

In that case **the tip is received in the external account holder's own account**, not in a Rakurai tip account. So Rakurai can't just drain it — instead the validator **records** the attributed amount every leader turn, and after the epoch the external account holder **settles** their agreed share into the Tips Collection Account PDA, from which the commission is deducted.

### Why a **Mev Share Collection Account** (MCA)

Same idea for [post-pack confirmations](../../../transaction_inclusion/post_pack_confirmations.md) used for MEV / arbitrage. The MEV-share revenue lands in the external account holder's own flow, so there's no account Rakurai can drain. We **trust the revenue source to share an agreed percentage** of MEV-share revenue; the validator records attribution per turn and the external account holder settles into the Mev Share Collection Account — exactly the same flow as tips, just a different `share_kind`.

### How Tip & MevShare are Distributed

Distribution works the same way for both a TCA and an MCA (only the `share_kind` differs).

The attributed amount is recorded in the account's ledger. After the epoch ends, the searcher transfers the recorded amount into the account. Once transferred, it is distributed in two parts:
 - **Client** (i.e. Rakurai): the client commission is credited to its account (the commission percentage is recorded in the account).
 - **Validator**: the remaining share is credited to its identity account.
   - The validator further has the option to convert the credited amount into block rewards. If enabled, once claimed, the claimed amount is converted into a high-priority block reward. The high-priority transaction is sent from the validator's identity account (because the amount was credited into the identity), and the transaction is guaranteed to land within the leader turn — it is created in the first turn of the leader slot, and the blockhash protects it so that if it does not land within those slots, it expires.

**Note:** if the external searcher/trader does not share revenue within 2 epochs, they will be disabled and will not be able to get custom tip prioritization or post-pack confirmation.

### Flow

| Step | Who | Instruction |
|------|-----|------|
| **Init** | anyone (payer) | `initialize_revenue_share_account` — enabled RAA for validator vote; once per `(share_kind, name, vote)` |
| **Record** | `record_authority` | `record_revenue(amount)` each leader turn — ledger only, no lamport move |
| **Settle** | searcher/trader | Post-epoch SOL transfer into the revenue-share PDA (≥ last epoch recorded amount) |
| **Claim** | `manager_authority` | `claim_revenue(epoch)` after epoch ends — splits `commission_bps` → `commission_account`, rest → validator identity |

### How to Check Status

The same steps apply to both a TCA and an MCA; only the `share_kind` in the seeds differs.
- The account struct is openly available, so you can decode it.
- Using Solscan, you can derive the address of the TCA/MCA.
- Use the Solscan PDA creation tool: https://solscan.io/tools#pda-create
- Seed: `[REVENUE_SHARE, share_kind ("TIP" | "MEV_SHARE"), name[32], validator_vote]`
  - Add the 4 seeds using the add button, and make sure to use the correct name and validator vote account.
  - This will give you the account (TCA/MCA) address, which you can then explore on Solscan to see its decoded data portion.

PDA: `[REVENUE_SHARE, share_kind ("TIP" \| "MEV_SHARE"), name[32], validator_vote]`.
`convert_to_block_rewards` is snapshotted into the ledger on the first `record_revenue` for each epoch.

---

## Account Lifecycle

### Reward Collection Account (RCA)
- `RewardCollectionAccount` is valid for **2 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator's identity account**.
  - The account is closed to reclaim rent.

### Tip & MevShare Collection Accounts (TCA & MCA)
- These accounts are created once, and only the manager authority can control them.
- They keep records for the most recent epochs, up to a configured capacity (`max_epoch_entries`, max 32); once full, the oldest epoch is overwritten.

---
