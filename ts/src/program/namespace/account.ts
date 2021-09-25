import camelCase from "camelcase";
import EventEmitter from "eventemitter3";
import * as bs58 from "bs58";
import {
  Signer,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  Commitment,
  GetProgramAccountsFilter,
} from "@solana/web3.js";
import Provider from "../../provider";
import { Idl, IdlTypeDef } from "../../idl";
import Coder, {
  ACCOUNT_DISCRIMINATOR_SIZE,
  accountSize,
  AccountsCoder,
} from "../../coder";
import { Subscription, Address, translateAddress } from "../common";
import { getProvider } from "../../";
import * as pubkeyUtil from "../../utils/pubkey";
import * as rpcUtil from "../../utils/rpc";

export default class AccountFactory {
  public static build(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    provider: Provider
  ): AccountNamespace {
    const accountFns: AccountNamespace = {};

    idl.accounts?.forEach((idlAccount) => {
      const name = camelCase(idlAccount.name);
      accountFns[name] = new AccountClient(
        idl,
        idlAccount,
        programId,
        provider,
        coder
      );
    });

    return accountFns;
  }
}

/**
 * The namespace provides handles to an [[AccountClient]] object for each
 * account in a program.
 *
 * ## Usage
 *
 * ```javascript
 * account.<account-client>
 * ```
 *
 * ## Example
 *
 * To fetch a `Counter` account from the above example,
 *
 * ```javascript
 * const counter = await program.account.counter.fetch(address);
 * ```
 *
 * For the full API, see the [[AccountClient]] reference.
 */
export interface AccountNamespace {
  [key: string]: AccountClient;
}

export class AccountClient<T = any> {
  /**
   * Returns the number of bytes in this account.
   */
  get size(): number {
    return this._size;
  }
  private _size: number;

  /**
   * Returns the program ID owning all accounts.
   */
  get programId(): PublicKey {
    return this._programId;
  }
  private _programId: PublicKey;

  /**
   * Returns the client's wallet and network provider.
   */
  get provider(): Provider {
    return this._provider;
  }
  private _provider: Provider;

  /**
   * Returns the coder.
   */
  get coder(): Coder {
    return this._coder;
  }
  private _coder: Coder;

  private _idlAccount: IdlTypeDef;

  constructor(
    idl: Idl,
    idlAccount: IdlTypeDef,
    programId: PublicKey,
    provider?: Provider,
    coder?: Coder
  ) {
    this._idlAccount = idlAccount;
    this._programId = programId;
    this._provider = provider ?? getProvider();
    this._coder = coder ?? new Coder(idl);
    this._size =
      ACCOUNT_DISCRIMINATOR_SIZE + (accountSize(idl, idlAccount) ?? 0);
  }

  /**
   * Returns a deserialized account, returning null if it doesn't exist.
   *
   * @param address The address of the account to fetch.
   */
  async fetchNullable(address: Address): Promise<T | null> {
    const accountInfo = await this._provider.connection.getAccountInfo(
      translateAddress(address)
    );
    if (accountInfo === null) {
      return null;
    }

    // Assert the account discriminator is correct.
    const discriminator = AccountsCoder.accountDiscriminator(
      this._idlAccount.name
    );
    if (discriminator.compare(accountInfo.data.slice(0, 8))) {
      throw new Error("Invalid account discriminator");
    }

    return this._coder.accounts.decode(this._idlAccount.name, accountInfo.data);
  }

  /**
   * Returns a deserialized account.
   *
   * @param address The address of the account to fetch.
   */
  async fetch(address: Address): Promise<T> {
    const data = await this.fetchNullable(address);
    if (data === null) {
      throw new Error(`Account does not exist ${address.toString()}`);
    }
    return data;
  }

  /**
   * Returns multiple deserialized accounts.
   * Accounts not found or with wrong discriminator are returned as null.
   *
   * @param addresses The addresses of the accounts to fetch.
   */
  async fetchMultiple(addresses: Address[]): Promise<(Object | null)[]> {
    const accounts = await rpcUtil.getMultipleAccounts(
      this._provider.connection,
      addresses.map((address) => translateAddress(address))
    );

    const discriminator = AccountsCoder.accountDiscriminator(
      this._idlAccount.name
    );
    // Decode accounts where discriminator is correct, null otherwise
    return accounts.map((account) => {
      if (account == null) {
        return null;
      }
      if (discriminator.compare(account?.account.data.slice(0, 8))) {
        return null;
      }
      return this._coder.accounts.decode(
        this._idlAccount.name,
        account?.account.data
      );
    });
  }

  /**
   * Returns all instances of this account type for the program.
   *
   * @param filters User-provided filters to narrow the results from `connection.getProgramAccounts`.
   *
   *                When filters are not defined this method returns all
   *                the account instances.
   *
   *                When filters are of type `Buffer`, the filters are appended
   *                after the discriminator.
   *
   *                When filters are of type `GetProgramAccountsFilter[]`,
   *                filters are appended after the discriminator filter.
   */
  async all(
    filters?: Buffer | GetProgramAccountsFilter[]
  ): Promise<ProgramAccount<T>[]> {
    const discriminator = AccountsCoder.accountDiscriminator(
      this._idlAccount.name
    );

    let resp = await this._provider.connection.getProgramAccounts(
      this._programId,
      {
        commitment: this._provider.connection.commitment,
        filters: [
          {
            memcmp: {
              offset: 0,
              bytes: bs58.encode(
                filters instanceof Buffer
                  ? Buffer.concat([discriminator, filters])
                  : discriminator
              ),
            },
          },
          ...(Array.isArray(filters) ? filters : []),
        ],
      }
    );
    return resp.map(({ pubkey, account }) => {
      return {
        publicKey: pubkey,
        account: this._coder.accounts.decode(
          this._idlAccount.name,
          account.data
        ),
      };
    });
  }

  /**
   * Returns an `EventEmitter` emitting a "change" event whenever the account
   * changes.
   */
  subscribe(address: Address, commitment?: Commitment): EventEmitter {
    const sub = subscriptions.get(address.toString());
    if (sub) {
      return sub.ee;
    }

    const ee = new EventEmitter();
    address = translateAddress(address);
    const listener = this._provider.connection.onAccountChange(
      address,
      (acc) => {
        const account = this._coder.accounts.decode(
          this._idlAccount.name,
          acc.data
        );
        ee.emit("change", account);
      },
      commitment
    );

    subscriptions.set(address.toString(), {
      ee,
      listener,
    });

    return ee;
  }

  /**
   * Unsubscribes from the account at the given address.
   */
  async unsubscribe(address: Address) {
    let sub = subscriptions.get(address.toString());
    if (!sub) {
      console.warn("Address is not subscribed");
      return;
    }
    if (subscriptions) {
      await this._provider.connection
        .removeAccountChangeListener(sub.listener)
        .then(() => {
          subscriptions.delete(address.toString());
        })
        .catch(console.error);
    }
  }

  /**
   * Returns an instruction for creating this account.
   */
  async createInstruction(
    signer: Signer,
    sizeOverride?: number
  ): Promise<TransactionInstruction> {
    const size = this.size;

    return SystemProgram.createAccount({
      fromPubkey: this._provider.wallet.publicKey,
      newAccountPubkey: signer.publicKey,
      space: sizeOverride ?? size,
      lamports: await this._provider.connection.getMinimumBalanceForRentExemption(
        sizeOverride ?? size
      ),
      programId: this._programId,
    });
  }

  /**
   * @deprecated since version 14.0.
   *
   * Function returning the associated account. Args are keys to associate.
   * Order matters.
   */
  async associated(...args: Array<PublicKey | Buffer>): Promise<T> {
    const addr = await this.associatedAddress(...args);
    return await this.fetch(addr);
  }

  /**
   * @deprecated since version 14.0.
   *
   * Function returning the associated address. Args are keys to associate.
   * Order matters.
   */
  async associatedAddress(
    ...args: Array<PublicKey | Buffer>
  ): Promise<PublicKey> {
    return await pubkeyUtil.associated(this._programId, ...args);
  }
}

/**
 * @hidden
 *
 * Deserialized account owned by a program.
 */
export type ProgramAccount<T = any> = {
  publicKey: PublicKey;
  account: T;
};

// Tracks all subscriptions.
const subscriptions: Map<string, Subscription> = new Map();
