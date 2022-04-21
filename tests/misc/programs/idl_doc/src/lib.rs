//! Testing the extraction of doc comments from the IDL.

use anchor_lang::prelude::*;


declare_id!("BqmKjZGVa8fqyWuojJzG16zaKSV1GjAisZToNuvEaz6m");

/// This is a doc comment for the program
#[program]
pub mod idl_doc {
    use super::*;

    /// This instruction doc should appear in the IDL
    pub fn test_idl_doc_parse(
        _ctx: Context<TestIdlDocParse>,
    ) -> Result<()> {
        Ok(())
    }
}

/// Custom account doc comment should appear in the IDL
#[account]
pub struct DataWithDoc {
    /// Account attribute doc comment should appear in the IDL
    pub data: u16,
}


#[derive(Accounts)]
pub struct TestIdlDocParse<'info> {
    /// This account doc comment should appear in the IDL
    /// This is a multi-line comment
    pub act: Account<'info, DataWithDoc>,
}
