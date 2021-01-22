#![feature(proc_macro_hygiene)]

// #region code
use anchor_lang::prelude::*;

#[program]
mod basic_4 {
    use super::*;

    #[state]
    pub struct MyProgram {
        pub data: u64,
    }

    impl MyProgram {
        pub fn new(data: u64) -> Result<Self, ProgramError> {
            Ok(Self { data })
        }
    }
}
// #endregion code
