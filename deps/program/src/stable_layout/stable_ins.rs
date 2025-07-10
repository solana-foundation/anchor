use crate::{
    account::AccountMeta, instruction::Instruction, pubkey::Pubkey,
    stable_layout::stable_vec::StableVec,
};

#[derive(Debug, PartialEq)]
#[repr(C)]
pub struct StableInstruction {
    pub program_id: Pubkey,
    pub accounts: StableVec<AccountMeta>,
    pub data: StableVec<u8>,
}

impl From<Instruction> for StableInstruction {
    fn from(other: Instruction) -> Self {
        Self {
            program_id: other.program_id,
            accounts: other.accounts.into(),
            data: other.data.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::account::AccountMeta;
    use crate::instruction::Instruction;
    use crate::pubkey::Pubkey;
    use crate::stable_layout::stable_vec::StableVec;

    #[test]
    fn test_instruction_to_stable_instruction() {
        let program_id = Pubkey::from([1u8; 32]);
        let account_meta = AccountMeta {
            pubkey: Pubkey::from([2u8; 32]),
            is_signer: true,
            is_writable: false,
        };
        let data = vec![0u8, 1, 2, 3];

        let instruction = Instruction {
            program_id,
            accounts: vec![account_meta.clone()],
            data: data.clone(),
        };

        let stable_instruction: StableInstruction = instruction.into();
        let expected_stable_instruction = StableInstruction {
            program_id,
            accounts: StableVec::from(vec![account_meta]),
            data: StableVec::from(data),
        };

        assert_eq!(stable_instruction, expected_stable_instruction);
    }
}
