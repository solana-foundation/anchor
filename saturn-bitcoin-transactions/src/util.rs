use arch_program::{account::AccountInfo, program::get_account_script_pubkey};
use bitcoin::Transaction;
use saturn_collections::generic::fixed_list::{FixedList, FixedListError};

/// Get the used shards in a transaction.
///
/// This function will reorder the accounts in the transaction to match the order of the accounts in the accounts list.
///
/// # Arguments
///
/// * `transaction` - The transaction to get the used shards from.
/// * `accounts` - The accounts to get the used shards from.
///
/// # Returns
///
/// A list of the used shards in the transaction.
///
/// # Example
///
/// ```
/// use saturn_bitcoin_transactions::util::get_used_shards_in_transaction;
/// use saturn_collections::generic::fixed_list::FixedList;
/// use arch_program::account::AccountInfo;
/// use bitcoin::{Transaction, transaction::Version, absolute::LockTime};
///
/// // Minimal empty transaction for demonstration purposes
/// # let transaction = Transaction {
/// #     version: Version::TWO,
/// #     lock_time: LockTime::ZERO,
/// #     input: vec![],
/// #     output: vec![],
/// # };
/// # // No accounts in this simple example
/// # let accounts: Vec<AccountInfo> = Vec::new();
/// let used_shards: FixedList<usize, 10> =
///     get_used_shards_in_transaction::<10>(&transaction, &accounts);
/// # assert!(used_shards.is_empty());
/// ```
pub fn get_used_shards_in_transaction<'a, const SIZE: usize>(
    transaction: &Transaction,
    accounts: &'a [AccountInfo<'a>],
) -> Result<FixedList<usize, SIZE>, FixedListError> {
    let mut used_shards = FixedList::<usize, SIZE>::new();

    for (index, account) in accounts.iter().enumerate() {
        if account.is_writable {
            used_shards.push(index)?;
        }
    }

    reorder_accounts_in_transaction(transaction, &mut used_shards, accounts)?;

    Ok(used_shards)
}

fn reorder_accounts_in_transaction<'a, const SIZE: usize>(
    transaction: &Transaction,
    account_indexes: &mut FixedList<usize, SIZE>,
    accounts: &'a [AccountInfo<'a>],
) -> Result<(), FixedListError> {
    let mut reordered_account_indexes = FixedList::<usize, SIZE>::new();

    // Iterate over transaction outputs and find matching accounts
    for output in &transaction.output {
        // Linear search instead of HashMap - more memory efficient for small collections
        for account_index in account_indexes.iter() {
            if *account_index >= accounts.len() {
                continue;
            }

            let account = &accounts[*account_index];
            let account_script_pubkey = get_account_script_pubkey(&account.key);

            // Compare script pubkeys directly without allocating ScriptBuf
            if output.script_pubkey.as_bytes() == account_script_pubkey {
                reordered_account_indexes.push(*account_index)?;
                break;
            }
        }
    }

    assert_eq!(
        reordered_account_indexes.len(),
        account_indexes.len(),
        "All accounts must be matched"
    );

    // Replace the original account_indexes with the reordered list
    account_indexes.copy_from_slice(reordered_account_indexes.as_slice());

    Ok(())
}
