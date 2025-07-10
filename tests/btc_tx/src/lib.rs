use anchor_lang::prelude::*;

declare_id!("11111111111111111111111111111111");

#[program(btc_tx(max_inputs_to_sign = 4, max_modified_accounts = 4, rune_capacity = 1))]
pub mod btc_tx_test_program {
    use super::*;

    pub fn demo(ctx: Context<Demo>) -> Result<()> {
        // Ensure we can access the builder and it is mutable
        let _builder = ctx.btc_tx_builder.ok_or(error!(ErrorCode::MissingBuilder))?;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Demo<'info> {
    pub signer: Signer<'info>,
}

#[error_code]
pub enum ErrorCode {
    #[msg("btc_tx_builder not present in Context")] 
    MissingBuilder,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compile_test() {
        // Just ensure the program module compiles and the demo handler type-checks.
        assert_eq!(crate::ID, crate::id());
    }
} 