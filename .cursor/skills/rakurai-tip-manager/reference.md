# Rakurai Tip Manager Reference

PDA seeds, account layouts, instruction accounts. Source: `programs/rakurai_tip_manager/src/`.

---

## PDA Seeds

| Account | Seeds |
|---------|-------|
| Config | `b"TIP_MANAGER_CONFIG_ACCOUNT"` |
| Tip 0–7 | `b"RAKURAI_TIP_ACCOUNT_N"` (N = 0..7) |

SDK: `derive_rakurai_tip_manager_config_account_address`, `derive_rakurai_tip_payment_account_pdas`, `derive_rakurai_partner_tip_share_address` (reward_distribution program + vote).

Partner share PDA (reward_distribution, Tip kind): `[PARTNER_SHARE, "TIP", "Rakurai"[32], validator_vote]`.

---

## Account Structs

### TipManagerConfigAccount

| Field | Type | Notes |
|-------|------|-------|
| authority | Pubkey | `close`, `change_block_builder` |
| validator_tip_receiver_account | Pubkey | Updated on `change_tip_receiver` → partner tip-share PDA |
| block_builder_commission_account | Pubkey | Commission destination |
| block_builder_commission_bps | u64 | 0–10000 |
| bumps | RakuraiTipManagerBumps | config + 8 tip bumps |

### RakuraiTipAccount

Empty state (`SIZE = 8` discriminator). Tips = lamports above rent.

---

## Instruction Accounts

### InitializeRakuraiTipManager

tip_manager_config (init), rakurai_tip_account_0..7 (init), system_program, payer (mut, signer)

### CloseRakuraiTipManager

tip_manager_config (mut, close→signer), rakurai_tip_account_0..7 (mut, close→signer), system_program, signer (mut, signer)

### ChangeTipReceiver

tip_manager_config (mut), rakurai_activation_account (RAA PDA), validator_vote_account, old_tip_receiver (mut), **reward_distribution_program**, new_tip_receiver (mut, partner share PDA, Tip kind), block_builder_commission_account (mut), rakurai_tip_account_0..7 (mut), signer (mut, signer)

**new_tip_receiver constraints:**
- `owner == reward_distribution_program.key`
- Address == `derive_rakurai_partner_tip_share_address(reward_distribution_program, vote)` = `[PARTNER_SHARE, "TIP", RAKURAI_PARTNER_TIP_SHARE_NAME, vote]`

Auth (in `auth()`): vote program owner; vote node == signer. RAA enabled + validator_authority via account constraints.

### ChangeBlockBuilder

tip_manager_config (mut), validator_tip_receiver_account (mut), old_block_builder (mut), new_block_builder (mut), rakurai_tip_account_0..7 (mut), signer (mut, signer)

---

## Drain Logic

1. Drain 8 PDAs → `total_tips` (preserve rent each)
2. `block_builder_fee = total * bps / 10000`
3. Credit **old** tip receiver + commission accounts (not `new_tip_receiver`)
4. `change_tip_receiver`: update `validator_tip_receiver_account` to `new_tip_receiver`

---

## Deployed Tip Account Addresses

Prefer SDK `derive_rakurai_tip_payment_account_pdas` for localnet/redeploy.

### Mainnet (`rKtiPTD7WuCdEEQ2JXWgAmZHHL9iZLc3niCXwtS7wSH`)

| Idx | Address |
|-----|---------|
| 0 | `BjqjPHFmwr19YFmkH8CMNJFbj1wzX9k9ngr4am2nQEdq` |
| 1 | `9CNKnAqJgLA4pL6KByzhhdY4mKoQP5wcPdhJgnvvi5Ve` |
| 2 | `5wy4C2VMFhHE4i8PWKNS1K4SV275zjNwhLwfKBwajrro` |
| 3 | `AgMdA97pk2i2Ry4YQ4iVPNrRiFhcH3x3ARUCiQGt3vJG` |
| 4 | `4Qf8JFV5vmpADXNouoJriQ9KiniT5DENrz9JM2mKGH9m` |
| 5 | `AuFAFzbzE9dzMajy4RNdyJZBTskeiuJQqT2wd9xoGSRD` |
| 6 | `8aLaHz8595MAvgxKoBJEyZmDfqQp8CorezFGYnC7CPjy` |
| 7 | `H6hyJo6rpBmwHbvVuWCEHExJ2bE4rcn1hTPeiBtypus4` |

### Testnet (`4qRZaFzf7MvgfBTCP9grb69cCST8UmKHPtkpGAgkJosD`)

| Idx | Address |
|-----|---------|
| 0 | `3ahyXyni1jLj8kJ13VgGEFDJzB374dgQW273nJSg8cdm` |
| 1 | `3aebD4TAn1somZfiaKRrMypUfmbDzT7XMVWRM5TFHuKW` |
| 2 | `Hm4LFyTAbrgH4eejYmNXQJ9oejQyq8frD2qeJbmkCAWR` |
| 3 | `AffPqNJ8jSrFGgfiouVfXcra1Vd6gHUjNhpoL8uW8dY5` |
| 4 | `9Z4pSxRZzE1T2e6587yzMWtvo8RHKW3R5Rb2FcprUPz` |
| 5 | `J2JdwcRrxWyCHKrgi2ipwCFXK2oRSgzPN4P7Q6Kz9XZ9` |
| 6 | `DscP7KHpAvfnboSKEQ5KEcwuFuRWn6MTjKYYTftuqY6z` |
| 7 | `Ur14r1oNyLvYeFLngGoEwYV4zwFVcui72vJqAavDXhZ` |

---

## Errors & Events

| Error | When |
|-------|------|
| ArithmeticError | Overflow in split/drain/close |
| MaxCommissionBpsExceeded | bps > 10_000 |
| Unauthorized | Wrong authority; vote mismatch; partner PDA mismatch |
| RakuraiSchedulerNotEnabled | RAA exists but `is_enabled == false` on `change_tip_receiver` |

Events: `TipsClaimedEvent` (drain ixs), `TipsManagerCloseEvent` (close).
