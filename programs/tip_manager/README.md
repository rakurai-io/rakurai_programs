# Rakurai Tip Manager Program

A Solana smart contract for managing tips sent to validators. The program maintains **eight tip accounts** to reduce write-lock contention and automatically splits tips between the validator's tip receiver account and the block builder commission account.

➤ For more details, refer to the [IDL File](./idl/tip_manager.json).

---

## How It Works

The Tip Manager Program uses a **singleton configuration account** (`TipManagerConfigAccount`) that controls:
- The **validator tip receiver account** — where validator tips are sent
- The **block builder commission account** — where block builder commission is sent
- The **block builder commission rate** (in basis points, 0–10000)

The program maintains **eight separate tip accounts** (PDAs) to minimize account write-lock contention when multiple transactions send tips simultaneously. Users can send tips to any of these eight accounts.

---

## Mainnet Deployment

**Tip Distribution Program ID:** `RktiPddFAPzG7CbgRtzVk64VE2RPxeUu2PbbeYov2Ne`

**Tip Accounts:**
- `68HZJtXe2JZebJayzr3S1c4GvkToDsbPqV5kEUicFzj7`
- `3GaWAjTUnnWawfjY9VAhdnFtRGS2mhEmPxLhp9Y1MiXU`
- `4fKa2igaUM1eRVDruwdebYVoTsGiZnt6wiZMaV5DEUZY`
- `fbmAcyyBK8nTFxMqrihznqB5F6C9Yd85oVbCQNVdX5F`
- `28pXrSbnwAqeMofX3xK5grWfZfpaLSCA8ypvBwrRGHGH`
- `Ec3HE4eZYig1vxQS2CWUvxKz5QPKdbceKdtzD3p8umQJ`
- `u663r6C8NBzUNSbTJoqxoEY1nKq7qWV2ZZWTtWSFLGf`
- `6mL1v8PBxnFGhEmf7j66NTsJZhNLUmU72aQFSMmzgQ5R`

---

## Account Structure

### TipManagerConfigAccount

A singleton PDA that stores the program configuration:

- **Fields**:
  - `authority` — Authorized updater of the config
  - `validator_tip_receiver_account` — Account receiving validator tips
  - `block_builder_commission_account` — Block builder commission account
  - `block_builder_commission_bps` — Commission in basis points (0–10000)
  - `bumps` — PDA bump seeds for all tip accounts

### RakuraiTipAccounts

These accounts are empty state accounts that hold SOL (lamports). When tips are claimed, all lamports above the rent-exempt minimum are drained and distributed.

---

## Tip Distribution Flow

1. **Users send tips** → Tips are sent to any of the eight tip accounts via standard SOL transfers
2. **Tips accumulate** → Tips accumulate in the tip accounts until claimed
3. **Validator claims tips** → Validator calls `change_tip_receiver` instruction to drain all accounts
4. **Automatic split** → Tips are automatically split:
   - Block builder commission → `block_builder_commission_account`
   - Remaining tips → `validator_tip_receiver_account`

---

## Integration with Reward Distribution

When validators receive tips through the Tip Manager Program, these tips are credited to their epoch specific **[Reward Collection Account (RCA)](../reward_distribution/README.md)**. The [`Reward Distribution Program`](../reward_distribution/README.md) then handles the distribution of these tips, including any commission deductions for Rakurai if applicable.

For more details on commission handling, see the [Reward Distribution Program documentation](../reward_distribution/README.md#rakurai-commission-on-mev-rewards).

---

## Account Lifecycle

- **Tip Accounts**: Remain open indefinitely, accumulating tips until drained
- **Config Account**: Remains open until explicitly closed by the authority
- All accounts preserve rent exemption when tips are drained

---
