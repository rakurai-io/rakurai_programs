# Rakurai Tip Manager Program

A Solana smart contract for managing tips sent to validators. The program maintains **eight tip accounts** to reduce write-lock contention and automatically splits tips between the validator's tip receiver account and the client commission account.

➤ For more details, refer to the [IDL file](./idl/rakurai_tip_manager.json).

---

## 1. Deployed program ID

### Mainnet

- **Tip Manager program:** [rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH](https://solscan.io/account/rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH)
- **Tip accounts:**
  - `BjqjPHFmwr19YFmkH8CMNJFbj1wzX9k9ngr4am2nQEdq`
  - `9CNKnAqJgLA4pL6KByzhhdY4mKoQP5wcPdhJgnvvi5Ve`
  - `5wy4C2VMFhHE4i8PWKNS1K4SV275zjNwhLwfKBwajrro`
  - `AgMdA97pk2i2Ry4YQ4iVPNrRiFhcH3x3ARUCiQGt3vJG`
  - `4Qf8JFV5vmpADXNouoJriQ9KiniT5DENrz9JM2mKGH9m`
  - `AuFAFzbzE9dzMajy4RNdyJZBTskeiuJQqT2wd9xoGSRD`
  - `8aLaHz8595MAvgxKoBJEyZmDfqQp8CorezFGYnC7CPjy`
  - `H6hyJo6rpBmwHbvVuWCEHExJ2bE4rcn1hTPeiBtypus4`

### Testnet

- **Tip Manager program:** [4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD](https://solscan.io/account/4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD?cluster=testnet)
- **Tip accounts:**
  - `3ahyXyni1jLj8kJ13VgGEFDJzB374dgQW273nJSg8cdm`
  - `3aebD4TAn1somZfiaKRrMypUfmbDzT7XMVWRM5TFHuKW`
  - `Hm4LFyTAbrgH4eejYmNXQJ9oejQyq8frD2qeJbmkCAWR`
  - `AffPqNJ8jSrFGgfiouVfXcra1Vd6gHUjNhpoL8uW8dY5`
  - `9Z4pSxRZzE1T2e6587yzMWtvo8RHKW3R5Rb2FcprUPz`
  - `J2JdwcRrxWyCHKrgi2ipwCFXK2oRSgzPN4P7Q6Kz9XZ9`
  - `DscP7KHpAvfnboSKEQ5KEcwuFuRWn6MTjKYYTftuqY6z`
  - `Ur14r1oNyLvYeFLngGoEwYV4zwFVcui72vJqAavDXhZ`

---

## 2. How it works

The Rakurai Tip Manager Program uses a **singleton configuration account** (`TipManagerConfigAccount`) that controls:

- The **validator tip receiver account** — where validator tips are sent
- The **client commission account** — where client commission is sent
- The **client commission rate** (in basis points, 0–10000)

The program maintains **eight separate tip accounts** (PDAs) to minimize account write-lock contention when multiple transactions send tips simultaneously. Users can send tips to any of these eight accounts.

---

## 3. Account structure

### 3.1. TipManagerConfigAccount

A singleton PDA that stores the program configuration:

- **Fields:**
  - `authority` — Authorized updater of the config
  - `validator_tip_receiver_account` — Account receiving validator tips
  - `client_commission_account` — Client commission account
  - `client_commission_bps` — Commission in basis points (0–10000)
  - `bumps` — PDA bump seeds for all tip accounts

### 3.2. RakuraiTipAccounts

These accounts are empty state accounts that hold SOL (lamports). When tips are claimed, all lamports above the rent-exempt minimum are drained and distributed.

---

## 4. Tip distribution flow

1. **Users send tips** → Tips are sent to any of the eight tip accounts via standard SOL transfers
2. **Tips accumulate** → Tips accumulate in the tip accounts until claimed
3. **Validator claims tips** → `change_tip_receiver_v1` (or legacy `change_tip_receiver`) drains tips and rotates config receiver to TCA
4. **Automatic split** → Tips are automatically split:
   - Client commission → `client_commission_account`
   - Remaining tips → `old_tip_receiver` (current config receiver)
5. **Config update** → `validator_tip_receiver_account` set to the Rakurai [Tips Collection Account](../reward_distribution/README.md#51-why-a-tips-collection-account-tca) (TCA) PDA for this validator vote

---

## 5. Integration with reward distribution

Tips land on the **TCA** via `change_tip_receiver_v1`.

Prerequisite: `initialize_revenue_share_account` (`share_kind = Tip`, name `"Rakurai"`) before the first drain. PDA: `[REVENUE_SHARE, "TIP", "Rakurai", validator_vote]`.

For the full TCA / MCA account layout, see [RevenueShareAccount structure](../reward_distribution/README.md#55-revenueshareaccount-structure).

---

## 6. Account lifecycle

- **Tip Accounts:** Remain open indefinitely, accumulating tips until drained
- **Config Account:** Remains open until explicitly closed by the authority
- All accounts preserve rent exemption when tips are drained

---
