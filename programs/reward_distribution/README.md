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
  - `client_commission_bps` — Commission (in basis points) for client (i.e., Rakurai) from block rewards.
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
  - **Client commission** → transferred to client (i.e., Rakurai) account.
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

Defaults for TCA/MCA init live on the **`TipsAndMevShareConfigAccount`** singleton (`initialize_revenue_share_account_v1`).

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
 - **Client** (i.e. Rakurai): share of the claim goes to `commission_account` at `commission_bps`.
 - **Validator**: the remaining share is credited to its identity account.
   - The validator further has the option to convert the credited amount into block rewards. If enabled, once claimed, the claimed amount is converted into a high-priority block reward. The high-priority transaction is sent from the validator's identity account (because the amount was credited into the identity), and the transaction is guaranteed to land within the leader turn — it is created in the first turn of the leader slot, and the blockhash protects it so that if it does not land within those slots, it expires.

**Rakurai vault claim exception:** when `share_kind == Tip` and `name == RAKURAI_REVENUE_NAME`, both `claim_revenue` and `claim_revenue_v1` force effective `commission_bps = 0`. Tip-manager drain (`change_tip_receiver_v1` / `v2`) already took Rakurai’s cut using tip-manager global commission (previous leader’s TCA terms); the vault only holds the validator share. Applying commission again at claim would double-charge. Partner / custom TCA and MCA vaults still apply `commission_bps` at claim.

**Note:** if the external searcher, trader, or transaction inclusion service does not share revenue within 2 epochs, they will be disabled and will not be able to get custom tip prioritization or post-pack confirmation.

Partners settle TCA/MCA balances with the [Partner Tip and MevShare Revenue Settlement CLI](../../cli/partner_reward_settlement.md) (`rakurai-partner-settle`: `get-account`, `get-pending-record`, `get-all-pending-records`, `transfer`).

### 5.4. Flow

| Step | Legacy TCA/MCA (`REVENUE_SHARE`) | TCAV1 / MCAV1 (`REVENUE_SHARE_V1`) |
|------|-----------------------------------|--------------------------------------|
| **Config** | RD config + full init args | One-time `initialize_tips_and_mev_share_config` |
| **Init** | `initialize_revenue_share_account` | `initialize_revenue_share_account_v1` |
| **Record** | `record_revenue` (amount only) | `record_revenue_v1` (Rakurai tip also credits `transferred_amount`) |
| **Settle** | N/A (claim needs vault lamports ≥ `amount`) | `settle_revenue` / `update_transferred_amount` |
| **Claim** | `claim_revenue` (pays `amount`) | `claim_revenue_v1` (pays `transferred_amount`; deficit; Rakurai name skips commission) |
| **Close** | `close_revenue_share_account` | `close_revenue_share_account_v1` |

### 5.5. TipsAndMevShareConfigAccount

Singleton PDA (`TIPS_AND_MEV_SHARE_CONFIG`) holding Tip and MevShare defaults copied onto TCA/MCA at `initialize_revenue_share_account_v1`:

| Side | Fields copied at init_v1 |
|------|--------------------------|
| Tip | `tip_manager_authority`, `tip_commission_account`, `tip_commission_bps`, `tip_epoch` → `max_epoch_entries` |
| MevShare | `mev_share_manager_authority`, `mev_share_commission_*`, `mev_share_epoch` |

`record_authority` is passed as an instruction argument to `initialize_revenue_share_account_v1` (same as legacy init).

Instructions: `initialize_tips_and_mev_share_config`, `update_tips_and_mev_share_config`, `close_tips_and_mev_share_config`.

### 5.6. RevenueShareAccount / RevenueShareAccountV1 structure

**Legacy** `RevenueShareAccount` (aliases TCA / MCA) and **V1** `RevenueShareAccountV1` (aliases TCAV1 / MCAV1) share the same header fields; V1 adds per-entry `transferred_amount` and account-level `deficit`. See the [Reward Distribution IDL](./idl/reward_distribution.json).

| Field | Type | Description |
|-------|------|-------------|
| `share_kind` | `RevenueKind` | `Tip` or `MevShare`; part of PDA seeds |
| `name` | `[u8; 32]` | UTF-8 padded label; Rakurai vaults use `RAKURAI_REVENUE_NAME` (`rakurai`) |
| `validator_vote` | `pubkey` | Validator vote account this account is tied to |
| `initializer` | `pubkey` | Account that paid to create the PDA; receives rent on close |
| `manager_authority` | `pubkey` | Signs `claim_revenue`, config updates, and close |
| `record_authority` | `pubkey` | Signs `record_revenue` — each leader turn for TCA; once post-epoch for MCA |
| `max_epoch_entries` | `u8` | Max distinct epochs stored in `ledger` (up to 32) |
| `commission_bps` | `u16` | Commission on claims; remainder goes to validator. Forced to 0 at claim when `name == RAKURAI_REVENUE_NAME` (Rakurai tip commission was already taken on tip-manager drain) |
| `commission_account` | `pubkey` | Receives the commission portion on claim |
| `block_reward_conversion_enabled` | `bool` | Whether claimed amounts can be converted into block rewards |
| `ledger` | `RevenueLedger` | Per-epoch attributed amounts |
| `deficit` | `u64` | Cumulative unpaid shortfall from underfunded claims; manager adjusts via `update_deficit` |
| `bump` | `u8` | PDA bump seed |

**`RevenueLedger` and `EpochAmountEntry`:**

```rust
pub struct RevenueLedger {
    pub entries: Vec<EpochAmountEntry>,
}

pub struct EpochAmountEntry {
    pub epoch: u64,                   // epoch this entry belongs to
    pub amount: u64,                  // attributed lamports (updated by record_revenue)
    pub transferred_amount: u64,      // settle_revenue, or auto on record_revenue for Rakurai tip TCA
    pub claimed: bool,                // true after claim_revenue succeeds
    pub block_reward_converted: bool, // whether converted to block rewards
}
```

Account-level `deficit: u64` (below the ledger) accumulates shortfalls when claim pays less than recorded (`amount - transferred`); manager adjusts via `update_deficit` with [`DeficitUpdate`].

**Example ledger after `record_revenue` + partial settle:**

```json
{
  "deficit": 0,
  "ledger": {
    "entries": [
      { "epoch": 998, "amount": 500000000, "transferred_amount": 300000000, "claimed": false, "block_reward_converted": false },
      { "epoch": 997, "amount": 1200000000, "transferred_amount": 1200000000, "claimed": true, "block_reward_converted": false }
    ]
  }
}
```

`record_revenue` / `record_revenue_v1` updates `amount`. For the **Rakurai tip TCAV1** only, `record_revenue_v1` also credits `transferred_amount` (tip-manager deposits SOL in the same drain tx). Non-Rakurai V1 vaults call `settle_revenue` (system-transfer + credit) or `update_transferred_amount` (credit only, after a direct SOL send). `claim_revenue_v1` pays `transferred_amount`, accrues `deficit` when underfunded (`amount > transferred`), and sets `claimed = true`. Legacy `claim_revenue` pays recorded `amount` and has no `deficit` / `transferred_amount`.

### 5.7. How to check status

- Solscan PDA tool: https://solscan.io/tools#pda-create
- **Legacy TCA/MCA:** `[REVENUE_SHARE, TIP|MEV_SHARE, name[32], validator_vote]`
- **TCAV1 / MCAV1:** `[REVENUE_SHARE_V1, TIP|MEV_SHARE, name[32], validator_vote]`

### 5.8. Dual vaults (legacy vs V1)

| | Seeds | Type | Tip manager |
|--|-------|------|-------------|
| Legacy | `[REVENUE_SHARE, TIP\|MEV_SHARE, name, vote]` | `RevenueShareAccount` | `change_tip_receiver_v1` + `record_revenue` |
| V1 | `[REVENUE_SHARE_V1, TIP\|MEV_SHARE, name, vote]` | `RevenueShareAccountV1` | `change_tip_receiver_v2` + `record_revenue_v1` |

Old validator releases keep using legacy PDAs and original ix names. New releases init TCAV1 and use `_v1` / TM `change_tip_receiver_v2`. Close unused legacy vaults with `close_revenue_share_account` when ready.

SDK: `derive_revenue_share_account_address` (legacy), `derive_revenue_share_account_v1_address` (V1).

---

## 6. Account lifecycle

### 6.1. Reward Collection Account (RCA)
- `RewardCollectionAccount` is valid for **2 epochs**.
- After that:
  - Any unclaimed funds are returned to the **validator's identity account**.
  - The account is closed to reclaim rent.

### 6.2. Tip and MevShare collection accounts

Legacy and V1 vaults coexist. Each is created once per `(seed space, share_kind, name, vote)`. Manager authority closes them when unused.

---
