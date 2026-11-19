use anchor_lang::{
    error::ErrorCode::AccountDidNotDeserialize,
    prelude::{AccountInfo, Pubkey, Result},
};
use bincode::deserialize;

pub struct VoteState;

impl VoteState {
    pub fn deserialize_node_pubkey(account_info: &AccountInfo) -> Result<Pubkey> {
        // The first 4 bytes are the enumeration type and the next 32 bytes of the vote state are the node pubkey.
        let data = account_info.data.borrow();
        let node_bytes = data.get(4..36).ok_or(AccountDidNotDeserialize)?;
        deserialize::<Pubkey>(node_bytes).map_err(|_| AccountDidNotDeserialize.into())
    }
}
