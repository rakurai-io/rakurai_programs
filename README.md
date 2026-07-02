# Rakurai Programs

A collection of Solana smart contracts and tools required for **Rakurai’s validator operations**.

## Documentation

| Section | Description |
| ------- | ----------- |
| [Rakurai Activation](./programs/rakurai_activation/README.md) | Multisig-controlled smart contract that authorizes and manages validators running Rakurai nodes. |
| [Reward Distribution](./programs/reward_distribution/README.md) | Distributes block rewards to stakers via post-epoch Merkle claims; tracks off-path tip and mev-share revenue in per-validator [Revenue Share Accounts](./programs/reward_distribution/README.md#revenue-share-accounts-tip--mevshare) (TCA / MCA). |
| [Rakurai Tip Manager](./programs/rakurai_tip_manager/README.md) | Manages tips sent to Rakurai validators across eight tip PDAs; drains and splits tips to the validator’s [Tips Collection Account](./programs/reward_distribution/README.md#why-a-tips-collection-account-tca) (TCA). |
| [Rakurai Activation CLI](./cli/README.md) | Command-line tool for interacting with the Rakurai Activation program — initialize a [Rakurai Activation Account](./programs/rakurai_activation/README.md#rakuraiactivationaccount-account-creation), enable/disable the scheduler, and update commission settings. |
