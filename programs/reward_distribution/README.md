# Rakurai Reward Distribution Program

A Solana smart contract for distributing block rewards among **Rakurai**, **validators**, and **stakers** via per-epoch **Reward Collection Accounts (RCA)** and post-epoch Merkle claims. It also tracks on-chain tip and MevShare revenue in per-validator, per-service **Tips Collection Accounts (TCA)** and **MevShare Collection Accounts (MCA)**.

➤ For more details, refer to the [IDL File](./idl/reward_distribution.json).

---

## 1. Deployed program ID

- **Mainnet**: [RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB](https://solscan.io/account/RAkd1EJg45QQHeuXy7JEWBhdNvsd64Z5PbZJWQT96iB)
- **Testnet**: [A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB](https://solscan.io/account/A37zgM34Q43gKAxBWQ9zSbQRRhjPqGK8jM49H7aWqNVB?cluster=testnet)

---

## 2. How it works

Each **validator**, for each **epoch**, creates a unique PDA called `RewardCollectionAccount`:

- **Seeds**: `["REWARD_COLLECTION_ACCOUNT", validator_vote_pubkey, epoch_number]`
- Only the validator's **authorized withdrawer** can initialize it.
- When creating the account, the validator must specify:
  - `reward_merkle_root_authority` — Authority responsible for uploading the Merkle root post-epoch.
  - `block_reward_commission_bps` — Commission (in basis points) that the validator retains from block rewards.
  - `client_commission_bps` — Commission (in basis points) for client (i.e: Rakurai) from block rewards.
  - `client_commission_account` — Destination account for client commission.

> The values for `client_commission_bps`, `block_reward_commission_bps`, and `client_commission_account` are pulled from the [RakuraiActivationAccount](../rakurai_activation/README.md#4-rakurai-activation-account-creation), a validator-specific PDA (not epoch-specific), part of the [`rakurai_activation`](../rakurai_activation/README.md) program. This account controls whether the validator is running the Rakurai scheduler (and should be charged commission).

---

## 3. Epoch flow

### 3.1. RewardCollectionAccount account initialization
On the first turn of each epoch, the `RewardCollectionAccount` is automatically initialized by the Rakurai Solana client. This initialization includes:
- Commission details (from validator-specific [`RakuraiActivationAccount`](../rakurai_activation/README.md)).
- Authority to update the reward Merkle root (only this authority can upload the Merkle root to the `RewardCollectionAccount` account).
> Account initialization logic is part of the Rakurai Solana client.


### 3.2. Per-turn transfers
During every leader turn:
- The **previous turn's block reward** is processed:
  - **Client commission** → transferred to client (i.e: Rakurai) account.
  - **Validator commission** → remains in the validator's identity account.
  - **Staker share** → accumulated into the `RewardCollectionAccount`.

> Because the reward of the current turn is transferred during the next one, the **first turn** of an epoch handles the **last reward** of the previous epoch.

---

### 3.3. Post-epoch staker distribution
At the final slot of each epoch, the following process takes place:
- A snapshot of Solana accounts is captured.
- Each validator's staker details and stake weights are extracted.
- An off-chain Merkle tree is generated containing reward share data.
  - **Extra flexibility**: At this stage, specific stakers can be blacklisted, and individual stake weights can be adjusted before finalizing the tree.
- The Merkle root is uploaded to the `RewardCollectionAccount` by the `reward_merkle_root_authority`.
- Stakers receive rewards via Merkle claims. When `reward_merkle_root_authority` is Rakurai, Rakurai runs the claim process on behalf of stakers.


## 4. Reward Distribution — Free and Automated by Rakurai

- Set the Merkle root authority to `--rewards-merkle-root-authority` to `H21wFgN53ghjDq5N9QhraAiPn1tRVYkobySj55unXLEj` for fully automated reward distribution.
- Keep it yourself if you want to run distribution manually.

When set to **Rakurai**, Rakurai will automatically:

1. **Create a snapshot**
2. **Calculate the Merkle root**
3. **Upload it on-chain**
4. **Run the claim process for stakers**

<p style="font-size:14px;">
    <span style="color:#66ff66;"><i><b>0% distribution fees charged by Rakurai</b></i></span>
    — only standard Solana transaction fees apply.
</p>

---

### 4.1. Client commission on MEV rewards

The client charges commission on MEV Rewards **only** if the following conditions are met:

- The validator is actively running **Rakurai during that epoch**.  
- The validator has set a non-zero **MEV commission** in their **Tip Distribution Account**.

*Note:* If the validator’s MEV commission is **0%**, Rakurai does **not** charge any commission on MEV tips.

---

### 4.2. Deduction flow

1. The validator’s share of MEV tips is credited to their **vote account** by the Tip Distribution Program in the following epoch.  
2. A `ClaimStatus` account is created to track that the validator has received MEV rewards.  
3. Rakurai client monitors the `ClaimStatus` account; once it is created, then it is eligible to deduct commission.  
4. Rakurai cannot deduct directly from the vote account, so the **same commission amount** is deducted from the validator’s **identity account** instead.  
5. The deduction is performed by invoking the `transfer_client_commission_on_mev_commission` instruction in the **Reward Distribution Program**.  
6. The commission rate is defined in the **Reward Distribution Config account**.

---

## 5. Tip and MevShare collection accounts

Most tip revenue flows through accounts controlled by the [Rakurai Tip Manager Program](../rakurai_tip_manager/README.md). For revenue that lands in accounts Rakurai does **not** control, this program tracks on-chain tip and MevShare revenue in per-validator, per-service **Tips Collection Accounts (TCA)** and **MevShare Collection Accounts (MCA)**.

Both use the same underlying `RevenueShareAccount`, parameterized by `share_kind ∈ {Tip, MevShare}` and exposed as the type aliases **`TipsCollectionAccount` (TCA)** and **`MevShareCollectionAccount` (MCA)**. Each account is uniquely tied to one validator, one searcher or transaction inclusion service, and one share kind — one TCA or MCA per `(service, validator)` pair.

### 5.1. Why a Tips Collection Account (TCA)

By default, searchers tip Rakurai's [eight tip accounts](../rakurai_tip_manager/README.md), and `rakurai_tip_manager` drains them automatically. But an external operator/searcher can [register their own custom tip account](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/rakurai_tip_manager_faqs) and agree to share a commission (e.g. 30%) with Rakurai.

In that case **the tip is received in the external account holder's own account**, not in a Rakurai tip account. So Rakurai can't just drain it — instead the validator **records** the attributed amount on-chain in the per-validator, per-service TCA ledger each leader turn, and after the epoch the external account holder **settles** their agreed share into the Tips Collection Account PDA, from which the commission is deducted.

### 5.2. Why a MevShare Collection Account (MCA)

Same idea for [post-pack confirmations](https://docs.rakurai.io/docs/services/rakurai_jito_private/rakurai_docs/transaction_inclusion/post_pack_confirmations) used for MEV / arbitrage. The MEV-share revenue lands in the searcher or transaction inclusion service's own flow, so there is no account Rakurai can drain. Rakurai tracks the agreed revenue share on-chain in a per-validator, per-service MCA.

Unlike TCA, **nothing is recorded in the MCA during leader turns**. After the epoch ends, the service **records** the owed amount in the MCA **once** via `record_revenue`, then **settles** by transferring SOL into the PDA.

### 5.3. How Tip and MevShare are distributed

**TCA (custom tips):** the validator **records** attributed amounts in the TCA ledger on each leader turn. After the epoch ends, the tip account holder **settles** by transferring SOL into the TCA, then `claim_revenue` distributes it.

**MCA (post-pack / MevShare):** nothing is recorded during leader turns. After the epoch ends, the service **records** the owed amount in the MCA **once**, **settles** by transferring SOL into the MCA, then `claim_revenue` distributes it.

Once settled (TCA or MCA), revenue is split in two parts:
 - **Client** (i.e. Rakurai): the client commission is credited to its account (the commission percentage is recorded in the account).
 - **Validator**: the remaining share is credited to its identity account.
   - The validator further has the option to convert the credited amount into block rewards. If enabled, once claimed, the claimed amount is converted into a high-priority block reward. The high-priority transaction is sent from the validator's identity account (because the amount was credited into the identity), and the transaction is guaranteed to land within the leader turn — it is created in the first turn of the leader slot, and the blockhash protects it so that if it does not land within those slots, it expires.

**Note:** if the external searcher, trader, or transaction inclusion service does not share revenue within 2 epochs, they will be disabled and will not be able to get custom tip prioritization or post-pack confirmation.

### 5.4. Flow

| Step | TCA (custom tips) | MCA (post-pack / MevShare) |
|------|-------------------|----------------------------|
| **Init** | `initialize_revenue_share_account` — once per `(share_kind, name, vote)` | Same |
| **Record** | Validator (`record_authority`) calls `record_revenue` **each leader turn** — ledger only | Service calls `record_revenue` **once after epoch end** — ledger only |
| **Settle** | Tip account holder transfers SOL into the PDA post-epoch | Service transfers SOL into the PDA post-epoch |
| **Claim** | `manager_authority` calls `claim_revenue(epoch)` — splits commission → `commission_account`, rest → validator identity | Same |

### 5.5. RevenueShareAccount structure

Both TCA and MCA use the same on-chain account type from the [Reward Distribution IDL](./idl/reward_distribution.json).

| Field | Type | Description |
|-------|------|-------------|
| `share_kind` | `RevenueKind` | `Tip` or `MevShare`; part of PDA seeds |
| `name` | `[u8; 32]` | UTF-8 padded UUID for the searcher or transaction inclusion service |
| `validator_vote` | `pubkey` | Validator vote account this account is tied to |
| `initializer` | `pubkey` | Account that paid to create the PDA; receives rent on close |
| `manager_authority` | `pubkey` | Signs `claim_revenue`, config updates, and close |
| `record_authority` | `pubkey` | Signs `record_revenue` — each leader turn for TCA; once post-epoch for MCA |
| `max_epoch_entries` | `u8` | Max distinct epochs stored in `ledger` (up to 32) |
| `commission_bps` | `u16` | Rakurai commission on claims; remainder goes to validator |
| `commission_account` | `pubkey` | Receives the commission portion on claim |
| `block_reward_conversion_enabled` | `bool` | Whether claimed amounts can be converted into block rewards |
| `ledger` | `RevenueLedger` | Per-epoch attributed amounts |
| `bump` | `u8` | PDA bump seed |

**`RevenueLedger` and `EpochAmountEntry`:**

```rust
pub struct RevenueLedger {
    pub entries: Vec<EpochAmountEntry>,
}

pub struct EpochAmountEntry {
    pub epoch: u64,                   // epoch this entry belongs to
    pub amount: u64,                  // attributed lamports (updated by record_revenue)
    pub claimed: bool,                // true after claim_revenue succeeds
    pub block_reward_converted: bool, // whether converted to block rewards
}
```

**Example ledger after `record_revenue`:**

```json
{
  "ledger": {
    "entries": [
      { "epoch": 998, "amount": 500000000, "claimed": false, "block_reward_converted": false },
      { "epoch": 997, "amount": 1200000000, "claimed": true, "block_reward_converted": false }
    ]
  }
}
```

`record_revenue` updates accounting only (no lamport move). For TCA it may be called each leader turn; for MCA the service calls it **once per epoch** after the epoch ends. Settlement is a separate SOL transfer into the PDA; `claim_revenue` distributes settled funds and sets `claimed = true`.

### 5.6. How to check status

TCA and MCA use the same account type; only `share_kind` in the seeds differs. Recording timing differs: TCA is updated each leader turn; MCA is updated once post-epoch by the service.
- The account struct is openly available, so you can decode it.
- Using Solscan, you can derive the address of the TCA/MCA.
- Use the Solscan PDA creation tool: https://solscan.io/tools#pda-create
- Seed: `[REVENUE_SHARE, share_kind ("TIP" | "MEV_SHARE"), name[32], validator_vote]`
  - Add the 4 seeds using the add button, and make sure to use the correct name and validator vote account.
  - This will give you the account (TCA/MCA) address, which you can then explore on Solscan to see its decoded data portion.

PDA: `[REVENUE_SHARE, share_kind ("TIP" \| "MEV_SHARE"), name[32], validator_vote]`.
`convert_to_block_rewards` is snapshotted into the ledger on the first `record_revenue` for each epoch.

---

## 6. Account lifecycle

### 6.1. Reward Collection Account (RCA)
- `RewardCollectionAccount` is valid for **2 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator's identity account**.
  - The account is closed to reclaim rent.

### 6.2. Tip and MevShare collection accounts (TCA and MCA)

Per-validator, per-service TCA and MCA accounts are created once, and only the manager authority can control them.

- They keep records for the most recent epochs, up to a configured capacity (`max_epoch_entries`, max 32); once full, the oldest epoch is overwritten.

---
