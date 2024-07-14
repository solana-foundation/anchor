use anchor_lang::prelude::*;

// Needed to declare accounts.
declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

mod inside_mod {
    use super::*;

    #[derive(InitSpace)]
    pub struct Data {
        pub data: u64,
    }
}

#[derive(InitSpace)]
pub enum TestBasicEnum {
    Basic1,
    Basic2 {
        test_u8: u8,
    },
    Basic3 {
        test_u16: u16,
    },
    Basic4 {
        #[max_len(10)]
        test_vec: Vec<u8>,
    },
}

#[account]
#[derive(InitSpace)]
pub struct TestEmptyAccount {}

#[account]
#[derive(InitSpace)]
pub struct TestBasicVarAccount {
    pub test_u8: u8,
    pub test_u16: u16,
    pub test_u32: u32,
    pub test_u64: u64,
    pub test_u128: u128,
}

#[account]
#[derive(InitSpace)]
pub struct TestComplexVarAccount {
    pub test_key: Pubkey,
    #[max_len(10)]
    pub test_vec: Vec<u8>,
    #[max_len(10)]
    pub test_string: String,
    pub test_option: Option<u16>,
}

#[derive(InitSpace)]
pub struct TestNonAccountStruct {
    pub test_bool: bool,
}

#[account(zero_copy)]
#[derive(InitSpace)]
pub struct TestZeroCopyStruct {
    pub test_array: [u8; 8],
    pub test_u32: u32,
}

#[derive(InitSpace)]
pub struct ChildStruct {
    #[max_len(10)]
    pub test_string: String,
}

#[derive(InitSpace)]
pub struct TestNestedStruct {
    pub test_struct: ChildStruct,
    pub test_enum: TestBasicEnum,
}

#[derive(InitSpace)]
pub struct TestMatrixStruct {
    #[max_len(2, 4)]
    pub test_matrix: Vec<Vec<u8>>,
}

#[derive(InitSpace)]
pub struct TestFullPath {
    pub test_option_path: Option<inside_mod::Data>,
    pub test_path: inside_mod::Data,
}

const MAX_LEN: u8 = 10;

#[derive(InitSpace)]
pub struct TestConst {
    #[max_len(MAX_LEN)]
    pub test_string: String,
    pub test_array: [u8; MAX_LEN as usize],
}

#[derive(InitSpace)]
pub struct TestUnnamedStruct(
    pub u8,
    #[max_len(4)] pub Vec<u32>,
    #[max_len(10)] pub String,
    pub ChildStruct,
    pub TestBasicEnum,
);

#[derive(InitSpace)]
pub struct TestUnitStruct;

#[test]
fn test_empty_struct() {
    assert_eq!(TestEmptyAccount::INIT_SPACE, 0);
}

#[test]
fn test_basic_struct() {
    assert_eq!(TestBasicVarAccount::INIT_SPACE, 1 + 2 + 4 + 8 + 16);
}

#[test]
fn test_complex_struct() {
    assert_eq!(
        TestComplexVarAccount::INIT_SPACE,
        32 + 4 + 10 + (4 + 10) + 3
    )
}

#[test]
fn test_zero_copy_struct() {
    assert_eq!(TestZeroCopyStruct::INIT_SPACE, 8 + 4)
}

#[test]
fn test_basic_enum() {
    assert_eq!(TestBasicEnum::INIT_SPACE, 1 + 14);
}

#[test]
fn test_nested_struct() {
    assert_eq!(
        TestNestedStruct::INIT_SPACE,
        ChildStruct::INIT_SPACE + TestBasicEnum::INIT_SPACE
    )
}

#[test]
fn test_matrix_struct() {
    assert_eq!(TestMatrixStruct::INIT_SPACE, 4 + (2 * (4 + 4)))
}

#[test]
fn test_full_path() {
    assert_eq!(TestFullPath::INIT_SPACE, 8 + 9)
}

#[test]
fn test_const() {
    assert_eq!(TestConst::INIT_SPACE, (4 + 10) + 10)
}

#[test]
fn test_unnamed_struct() {
    assert_eq!(
        TestUnnamedStruct::INIT_SPACE,
        1 + 4 + 4 * 4 + 4 + 10 + ChildStruct::INIT_SPACE + TestBasicEnum::INIT_SPACE
    )
}

#[test]
fn test_unit_struct() {
    assert_eq!(TestUnitStruct::INIT_SPACE, 0)
}
