# Rakurai Programs

A collection of Solana smart contracts and tools required for **Rakurai’s validator operations**.

**Rakurai Activation**  
- Multisig-controlled smart contract that authorizes and manages validators running Rakurai nodes. [Details](./programs/rakurai_activation/README.md)

**Reward Distribution**  
- Smart contract for distributing block rewards to stakers using a permissionless, Merkle-root-based verification model. [Details](./programs/reward_distribution/README.md)

**Rakurai Tip Manager**  
- Smart contract for managing tips sent to validators. Maintains eight tip accounts to reduce write-lock contention and automatically splits tips between the validator's tip receiver account and the block builder commission account. [Details](./programs/tip_manager/README.md)

**Rakurai Activation CLI**  
- Command-line tool for interacting with the Rakurai Activation program.  
  Allows validators to make key changes to their [RakuraiActivationAccount](./programs/rakurai_activation/README.md#rakuraiactivationaccount-account-creation) — such as enabling/disabling a rakurai scheduler or updating its commission rate. [Details](./cli/README.md)