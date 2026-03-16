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

**Tip Distribution Program ID:** `rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH`

**Tip Accounts:**
- `BjqjPHFmwr19YFmkH8CMNJFbj1wzX9k9ngr4am2nQEdq`
- `9CNKnAqJgLA4pL6KByzhhdY4mKoQP5wcPdhJgnvvi5Ve`
- `5wy4C2VMFhHE4i8PWKNS1K4SV275zjNwhLwfKBwajrro`
- `AgMdA97pk2i2Ry4YQ4iVPNrRiFhcH3x3ARUCiQGt3vJG`
- `4Qf8JFV5vmpADXNouoJriQ9KiniT5DENrz9JM2mKGH9m`
- `AuFAFzbzE9dzMajy4RNdyJZBTskeiuJQqT2wd9xoGSRD`
- `8aLaHz8595MAvgxKoBJEyZmDfqQp8CorezFGYnC7CPjy`
- `H6hyJo6rpBmwHbvVuWCEHExJ2bE4rcn1hTPeiBtypus4`

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

---

## Account Lifecycle

- **Tip Accounts**: Remain open indefinitely, accumulating tips until drained
- **Config Account**: Remains open until explicitly closed by the authority
- All accounts preserve rent exemption when tips are drained

---
