# Rakurai Activation Reference

PDA seeds, account layouts, instruction matrices. Source: `programs/rakurai_activation/src/`.

---

## PDA Seeds

| Account | Seeds |
|---------|-------|
| Config | `b"ACTIVATION_CONFIG_ACCOUNT"` |
| RAA | `b"RAKURAI_ACTIVATION_ACCOUNT"`, `validator_identity` |

SDK: `derive_config_account_address`, `derive_activation_account_address`.

---

## Account Structs

### RakuraiActivationConfigAccount

| Field | Type | Notes |
|-------|------|-------|
| authority | Pubkey | `update_config` |
| block_builder_authority | Pubkey | Scheduler co-signer; block-builder commission updates |
| block_builder_commission_bps | u16 | 0–10000 |
| block_builder_commission_account | Pubkey | Non-default required |
| bump | u8 | |

### RakuraiActivationAccount

| Field | Type | Notes |
|-------|------|-------|
| is_enabled | bool | Scheduler active |
| proposer | Option\<Pubkey\> | Pending multisig proposer |
| validator_authority | Pubkey | Identity |
| block_reward_commission_bps | u16 | Validator retention |
| block_builder_commission_bps | u16 | Per-RAA; block builder can update |
| bump | u8 | |
| hash | Option\<[u8; 64]\> | Block builder scheduler hash |

---

## Instructions — Signers

| Instruction | Signers |
|-------------|---------|
| initialize | initializer (payer) |
| update_config | authority |
| initialize_rakurai_activation_account | validator identity |
| update_rakurai_activation_approval | validator or block builder |
| update_rakurai_activation_commission | validator or block builder |
| close_rakurai_activation_account | block_builder_authority |

---

## Instruction Accounts

**Initialize**: config (init), system_program, initializer (mut, signer)

**UpdateConfig**: config (mut), authority (mut, signer)

**InitializeRakuraiActivationAccount**: config, activation_account (init), validator_vote_account, validator_identity_account, signer (mut, signer), system_program

**UpdateRakuraiActivationApproval**: config, activation_account (mut), validator_identity_account, signer (mut, signer)

**UpdateRakuraiActivationCommission**: config, activation_account (mut), validator_identity_account, signer (mut, signer)

**CloseRakuraiActivationAccount**: config, activation_account (mut, close→identity), validator_identity_account (mut), signer (mut, signer)

---

## Multisig State Machine (`update_rakurai_activation_approval`)

`grant_approval = false`: `is_enabled = false`, `hash = None`, `proposer = None`.

`grant_approval = true`, `is_enabled = false`:

| proposer | Signer | Effect |
|----------|--------|--------|
| None | validator | `proposer = signer` |
| None | block builder | `proposer = signer`, hash required |
| Some(s) == signer | either | no-op ("Proposal Pending") |
| Some(s) != signer | validator | `proposer = None`, `is_enabled = true` |
| Some(s) != signer | block builder | hash required; then enable |

`grant_approval = true`, `is_enabled = true`, block builder signer: updates `hash` only.

---

## Error Codes

| Code | When |
|------|------|
| AccountValidationFailure | Invalid pubkeys / validate() |
| MaxCommissionBpsExceeded | bps > 10_000 |
| MissingHashForEnable | Block builder enable without hash |
| Unauthorized | Wrong signer / vote owner mismatch |
| ArithmeticError | Lamport overflow on close |

---

## Events

`ConfigUpdatedEvent`, `RakuraiActivationAccountInitializedEvent`, `UpdateRakuraiActivationApprovalEvent`, `UpdateRakuraiActivationCommissionEvent`, `RakuraiActivationAccountClosedEvent`.
