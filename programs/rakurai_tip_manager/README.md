# Rakurai Tip Manager Program

A Solana smart contract for managing tips sent to validators. The program maintains **eight tip accounts** to reduce write-lock contention and automatically splits tips between the validator's tip receiver account and the client commission account.

➤ For more details, refer to the [IDL File](./idl/rakurai_tip_manager.json).

---
### Deployed Program ID

| Network | Tip Manager Program | Tip accounts |
|---------|---------------------|--------------|
| **Mainnet** | [rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH](https://solscan.io/account/rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH) | `BjqjPHFmwr19YFmkH8CMNJFbj1wzX9k9ngr4am2nQEdq`<br>`9CNKnAqJgLA4pL6KByzhhdY4mKoQP5wcPdhJgnvvi5Ve`<br>`5wy4C2VMFhHE4i8PWKNS1K4SV275zjNwhLwfKBwajrro`<br>`AgMdA97pk2i2Ry4YQ4iVPNrRiFhcH3x3ARUCiQGt3vJG`<br>`4Qf8JFV5vmpADXNouoJriQ9KiniT5DENrz9JM2mKGH9m`<br>`AuFAFzbzE9dzMajy4RNdyJZBTskeiuJQqT2wd9xoGSRD`<br>`8aLaHz8595MAvgxKoBJEyZmDfqQp8CorezFGYnC7CPjy`<br>`H6hyJo6rpBmwHbvVuWCEHExJ2bE4rcn1hTPeiBtypus4` |
| **Testnet** | [4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD](https://solscan.io/account/4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD?cluster=testnet) | `3ahyXyni1jLj8kJ13VgGEFDJzB374dgQW273nJSg8cdm`<br>`3aebD4TAn1somZfiaKRrMypUfmbDzT7XMVWRM5TFHuKW`<br>`Hm4LFyTAbrgH4eejYmNXQJ9oejQyq8frD2qeJbmkCAWR`<br>`AffPqNJ8jSrFGgfiouVfXcra1Vd6gHUjNhpoL8uW8dY5`<br>`9Z4pSxRZzE1T2e6587yzMWtvo8RHKW3R5Rb2FcprUPz`<br>`J2JdwcRrxWyCHKrgi2ipwCFXK2oRSgzPN4P7Q6Kz9XZ9`<br>`DscP7KHpAvfnboSKEQ5KEcwuFuRWn6MTjKYYTftuqY6z`<br>`Ur14r1oNyLvYeFLngGoEwYV4zwFVcui72vJqAavDXhZ` |

---

## How It Works

The Rakurai Tip Manager Program uses a **singleton configuration account** (`TipManagerConfigAccount`) that controls:
- The **validator tip receiver account** — where validator tips are sent
- The **client commission account** — where client commission is sent
- The **client commission rate** (in basis points, 0–10000)

The program maintains **eight separate tip accounts** (PDAs) to minimize account write-lock contention when multiple transactions send tips simultaneously. Users can send tips to any of these eight accounts.

---

## Account Structure

### TipManagerConfigAccount

A singleton PDA that stores the program configuration:

- **Fields**:
  - `authority` — Authorized updater of the config
  - `validator_tip_receiver_account` — Account receiving validator tips
  - `client_commission_account` — client commission account
  - `client_commission_bps` — Commission in basis points (0–10000)
  - `bumps` — PDA bump seeds for all tip accounts

### RakuraiTipAccounts

These accounts are empty state accounts that hold SOL (lamports). When tips are claimed, all lamports above the rent-exempt minimum are drained and distributed.

---

## Tip Distribution Flow

1. **Users send tips** → Tips are sent to any of the eight tip accounts via standard SOL transfers
2. **Tips accumulate** → Tips accumulate in the tip accounts until claimed
3. **Validator claims tips** → `change_tip_receiver_v1` (or legacy `change_tip_receiver`) drains tips and rotates config receiver to TCA
4. **Automatic split** → Tips are automatically split:
   - client commission → `client_commission_account`
   - Remaining tips → `old_tip_receiver` (current config receiver)
5. **Config update** → `validator_tip_receiver_account` set to the Rakurai [Tips Collection Account](../reward_distribution/README.md#why-a-tips-collection-account-tca) (TCA) PDA for this validator vote

---

## Integration with Reward Distribution

Tips land on the **TCA** via `change_tip_receiver_v1`.

Prerequisite: `initialize_revenue_share_account` (`share_kind = Tip`, name `"Rakurai"`) before the first drain. PDA: `[REVENUE_SHARE, "TIP", "Rakurai", validator_vote]`.

---

## Account Lifecycle

- **Tip Accounts**: Remain open indefinitely, accumulating tips until drained
- **Config Account**: Remains open until explicitly closed by the authority
- All accounts preserve rent exemption when tips are drained

---
