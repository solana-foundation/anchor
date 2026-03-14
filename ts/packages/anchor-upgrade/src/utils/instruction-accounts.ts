import type { IdlInstructionAccountItem, IdlInstructionAccounts } from '../types';

export type FlatInstructionAccount = {
  name: string;
};

export function isCompositeAccounts(
  accountItem: IdlInstructionAccountItem,
): accountItem is IdlInstructionAccounts {
  return 'accounts' in accountItem;
}

export function flattenInstructionAccounts(
  accounts: IdlInstructionAccountItem[],
): FlatInstructionAccount[] {
  return accounts.flatMap((accountItem) => {
    if (isCompositeAccounts(accountItem)) {
      return flattenInstructionAccounts(accountItem.accounts);
    }

    return [{ name: accountItem.name }];
  });
}
