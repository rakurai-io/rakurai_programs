use solana_sdk::{account::Account, pubkey::Pubkey, vote::program as vote_program};

/// Minimal vote account: 4-byte enum tag + 32-byte node pubkey at offset 4.
/// Programs only read bytes `[4..36]` via `VoteState::deserialize_node_pubkey`.
pub fn mock_vote_account(node_pubkey: &Pubkey, lamports: u64) -> Account {
    let mut data = vec![0u8; 3762];
    data[4..36].copy_from_slice(node_pubkey.as_ref());
    Account {
        lamports,
        data,
        owner: vote_program::id(),
        executable: false,
        rent_epoch: 0,
    }
}
