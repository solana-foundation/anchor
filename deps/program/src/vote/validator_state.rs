use crate::pubkey::Pubkey;

use super::program::VOTE_PROGRAM_ID;

pub const VALIDATOR_STATE_SEED_PREFIX: &[u8] = b"validator-state";

#[derive(Default, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SharedValidatorState {
    /// The account that will hold the whitelist, we check it against the bootnode input to the validator
    pub bootnode_pubkey: Pubkey,

    /// The pubkey package of the network
    pub pubkey_package: Vec<u8>,

    /// The whitelist of the network
    pub whitelist: Vec<Pubkey>,
}

impl SharedValidatorState {
    pub fn new(bootnode_pubkey: Pubkey, pubkey_package: Vec<u8>, whitelist: Vec<Pubkey>) -> Self {
        Self {
            bootnode_pubkey,
            pubkey_package,
            whitelist,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }

    pub fn deserialize(data: &[u8]) -> Self {
        bincode::deserialize(data).unwrap()
    }
}

pub fn get_validator_state_account_pubkey_and_bump() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[VALIDATOR_STATE_SEED_PREFIX], &VOTE_PROGRAM_ID)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_state_address_consistency() {
        let (address1, bump1) = get_validator_state_account_pubkey_and_bump();
        let (address2, bump2) = get_validator_state_account_pubkey_and_bump();
        let (address3, bump3) = get_validator_state_account_pubkey_and_bump();

        assert_eq!(address1, address2);
        assert_eq!(address2, address3);

        assert_eq!(bump1, bump2);
        assert_eq!(bump2, bump3);

        println!("Validator state address: {:?}", address1);

        let (expected_pda, _bump) =
            Pubkey::find_program_address(&[b"validator-state"], &VOTE_PROGRAM_ID);
        assert_eq!(address1, expected_pda);
    }

    #[test]
    fn test_validator_state_address_components() {
        let (key, bump) = get_validator_state_account_pubkey_and_bump();

        // Manually derive the PDA to verify components
        let (pda, bump) = Pubkey::find_program_address(&[b"validator-state"], &VOTE_PROGRAM_ID);

        // Verify the address matches manual derivation
        assert_eq!(key, pda);

        // Verify the address is not the vote program itself
        assert_ne!(key, VOTE_PROGRAM_ID);

        println!("Validator state PDA: {:?}", pda);
        println!("Bump seed used: {}", bump);
    }
}
