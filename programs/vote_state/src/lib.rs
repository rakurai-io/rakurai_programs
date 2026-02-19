use anchor_lang::{
    error::ErrorCode::{AccountDidNotDeserialize, ConstraintOwner},
    prelude::{AccountInfo, Pubkey, Result},
};
use bincode::deserialize;

pub struct VoteState;

impl VoteState {
    pub fn deserialize_node_pubkey(account_info: &AccountInfo) -> Result<Pubkey> {
        // The first 4 bytes are the enumeration type and the next 32 bytes of the vote state are the node pubkey.
        let data = account_info.data.borrow();
        deserialize::<Pubkey>(&data[4..36]).map_err(|_| AccountDidNotDeserialize.into())
    }
}
