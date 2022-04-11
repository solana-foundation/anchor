import camelCase from "camelcase";
import EventEmitter from "eventemitter3";
import {
  Signer,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
  Commitment,
  GetProgramAccountsFilter,
  AccountInfo,
} from "@solana/web3.js";
import Provider, { getProvider } from "../../provider.js";
import { Idl, IdlAccountDef } from "../../idl.js";
import { Coder, BorshCoder } from "../../coder/index.js";
import { Subscription, Address, translateAddress } from "../common.js";
import { AllAccountsMap, IdlTypes, TypeDef } from "./types.js";
import * as pubkeyUtil from "../../utils/pubkey.js";
import * as rpcUtil from "../../utils/rpc.js";

export default class AccountFactory {
  public static build<IDL extends Idl>(
    idl: IDL,
    coder: Coder,
    programId: PublicKey,
    provider?: Provider
  ): AccountNamespace<IDL> {
    const accountFns: AccountNamespace = {};

    idl.accounts?.forEach((idlAccount) => {
      const name = camelCase(idlAccount.name);
      accountFns[name] = new AccountClient<IDL>(
        idl,
        idlAccount,
        programId,
        provider,
        coder
      );
    });

    return accountFns as AccountNamespace<IDL>;
  }
}

type NullableIdlAccount<IDL extends Idl> = IDL["accounts"] extends undefined
  ? IdlAccountDef
  : NonNullable<IDL["accounts"]>[number];

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
export type AccountNamespace<IDL extends Idl = Idl> = {
  [M in keyof AllAccountsMap<IDL>]: AccountClient<IDL>;
};

export class AccountClient<
  IDL extends Idl = Idl,
  A extends NullableIdlAccount<IDL> = IDL["accounts"] extends undefined
    ? IdlAccountDef
    : NonNullable<IDL["accounts"]>[number],
  T = TypeDef<A, IdlTypes<IDL>>
> {
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

  private _idlAccount: A;

  constructor(
    idl: IDL,
    idlAccount: A,
    programId: PublicKey,
    provider?: Provider,
    coder?: Coder
  ) {
    this._idlAccount = idlAccount;
    this._programId = programId;
    this._provider = provider ?? getProvider();
    this._coder = coder ?? new BorshCoder(idl);
    this._size = this._coder.accounts.size(idlAccount);
  }

  /**
   * Returns a deserialized account, returning null if it doesn't exist.
   *
   * @param address The address of the account to fetch.
   */
  async fetchNullable(
    address: Address,
    commitment?: Commitment
  ): Promise<T | null> {
    const accountInfo = await this.getAccountInfo(address, commitment);
    if (accountInfo === null) {
      return null;
    }
    return this._coder.accounts.decode<T>(
      this._idlAccount.name,
      accountInfo.data
    );
  }

  /**
   * Returns a deserialized account.
   *
   * @param address The address of the account to fetch.
   */
  async fetch(address: Address, commitment?: Commitment): Promise<T> {
    const data = await this.fetchNullable(address, commitment);
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
  async fetchMultiple(
    addresses: Address[],
    commitment?: Commitment
  ): Promise<(Object | null)[]> {
    const accounts = await rpcUtil.getMultipleAccounts(
      this._provider.connection,
      addresses.map((address) => translateAddress(address)),
      commitment
    );

    // Decode accounts where discriminator is correct, null otherwise
    return accounts.map((account) => {
      if (account == null) {
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
    let resp = await this._provider.connection.getProgramAccounts(
      this._programId,
      {
        commitment: this._provider.connection.commitment,
        filters: [
          {
            memcmp: this.coder.accounts.memcmp(
              this._idlAccount.name,
              filters instanceof Buffer ? filters : undefined
            ),
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

    // @ts-expect-error
    if (this._provider.wallet === undefined) {
      throw new Error(
        "This function requires the Provider interface implementor to have a 'wallet' field."
      );
    }

    return SystemProgram.createAccount({
      // @ts-expect-error
      fromPubkey: this._provider.wallet.publicKey,
      newAccountPubkey: signer.publicKey,
      space: sizeOverride ?? size,
      lamports:
        await this._provider.connection.getMinimumBalanceForRentExemption(
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

  async getAccountInfo(
    address: Address,
    commitment?: Commitment
  ): Promise<AccountInfo<Buffer> | null> {
    return await this._provider.connection.getAccountInfo(
      translateAddress(address),
      commitment
    );
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
