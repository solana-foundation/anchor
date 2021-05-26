import EventEmitter from "eventemitter3";
import camelCase from "camelcase";
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Commitment,
} from "@solana/web3.js";
import Provider from "../../provider";
import { Idl, IdlStateMethod } from "../../idl";
import Coder, { stateDiscriminator } from "../../coder";
import { RpcNamespace, InstructionNamespace, TransactionNamespace } from "./";
import { getProvider } from "../../";
import { Subscription, validateAccounts, parseIdlErrors } from "../common";
import { findProgramAddressSync, createWithSeedSync } from "../../utils/pubkey";
import { Accounts } from "../context";
import InstructionNamespaceFactory from "./instruction";
import RpcNamespaceFactory from "./rpc";
import TransactionNamespaceFactory from "./transaction";

export default class StateFactory {
  public static build(
    idl: Idl,
    coder: Coder,
    programId: PublicKey,
    provider: Provider
  ): StateClient | undefined {
    if (idl.state === undefined) {
      return undefined;
    }
    return new StateClient(idl, programId, provider, coder);
  }
}

/**
 * A client for the program state. Similar to the base [[Program]] client,
 * one can use this to send transactions and read accounts for the state
 * abstraction.
 */
export class StateClient {
  /**
   * [[RpcNamespace]] for all state methods.
   */
  readonly rpc: RpcNamespace;

  /**
   * [[InstructionNamespace]] for all state methods.
   */
  readonly instruction: InstructionNamespace;

  /**
   * [[TransactionNamespace]] for all state methods.
   */
  readonly transaction: TransactionNamespace;

  /**
   * Returns the program ID owning the state.
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

  private _address: PublicKey;
  private _coder: Coder;
  private _idl: Idl;
  private _sub: Subscription | null;

  constructor(
    idl: Idl,
    programId: PublicKey,
    provider?: Provider,
    coder?: Coder
  ) {
    this._idl = idl;
    this._programId = programId;
    this._address = programStateAddress(programId);
    this._provider = provider ?? getProvider();
    this._coder = coder ?? new Coder(idl);
    this._sub = null;

    // Build namespaces.
    const [instruction, transaction, rpc] = ((): [
      InstructionNamespace,
      TransactionNamespace,
      RpcNamespace
    ] => {
      let instruction: InstructionNamespace = {};
      let transaction: TransactionNamespace = {};
      let rpc: RpcNamespace = {};

      idl.state.methods.forEach((m: IdlStateMethod) => {
        // Build instruction method.
        const ixItem = InstructionNamespaceFactory.build(
          m,
          (ixName: string, ix: any) =>
            coder.instruction.encodeState(ixName, ix),
          programId
        );
        ixItem["accounts"] = (accounts: Accounts) => {
          const keys = stateInstructionKeys(programId, provider, m, accounts);
          return keys.concat(
            InstructionNamespaceFactory.accountsArray(accounts, m.accounts)
          );
        };
        // Build transaction method.
        const txItem = TransactionNamespaceFactory.build(m, ixItem);
        // Build RPC method.
        const rpcItem = RpcNamespaceFactory.build(
          m,
          txItem,
          parseIdlErrors(idl),
          provider
        );

        // Attach them all to their respective namespaces.
        const name = camelCase(m.name);
        instruction[name] = ixItem;
        transaction[name] = txItem;
        rpc[name] = rpcItem;
      });

      return [instruction, transaction, rpc];
    })();
    this.instruction = instruction;
    this.transaction = transaction;
    this.rpc = rpc;
  }

  /**
   * Returns the deserialized state account.
   */
  async fetch(): Promise<Object> {
    const addr = this.address();
    const accountInfo = await this.provider.connection.getAccountInfo(addr);
    if (accountInfo === null) {
      throw new Error(`Account does not exist ${addr.toString()}`);
    }
    // Assert the account discriminator is correct.
    const expectedDiscriminator = await stateDiscriminator(
      this._idl.state.struct.name
    );
    if (expectedDiscriminator.compare(accountInfo.data.slice(0, 8))) {
      throw new Error("Invalid account discriminator");
    }
    return this.coder.state.decode(accountInfo.data);
  }

  /**
   * Returns the state address.
   */
  address(): PublicKey {
    return this._address;
  }

  /**
   * Returns an `EventEmitter` with a `"change"` event that's fired whenever
   * the state account cahnges.
   */
  subscribe(commitment?: Commitment): EventEmitter {
    if (this._sub !== null) {
      return this._sub.ee;
    }
    const ee = new EventEmitter();

    const listener = this.provider.connection.onAccountChange(
      this.address(),
      (acc) => {
        const account = this.coder.state.decode(acc.data);
        ee.emit("change", account);
      },
      commitment
    );

    this._sub = {
      ee,
      listener,
    };

    return ee;
  }

  /**
   * Unsubscribes to state changes.
   */
  unsubscribe() {
    if (this._sub !== null) {
      this.provider.connection
        .removeAccountChangeListener(this._sub.listener)
        .then(async () => {
          this._sub = null;
        })
        .catch(console.error);
    }
  }
}

// Calculates the deterministic address of the program's "state" account.
function programStateAddress(programId: PublicKey): PublicKey {
  let [registrySigner] = findProgramAddressSync([], programId);
  return createWithSeedSync(registrySigner, "unversioned", programId);
}

// Returns the common keys that are prepended to all instructions targeting
// the "state" of a program.
function stateInstructionKeys(
  programId: PublicKey,
  provider: Provider,
  m: IdlStateMethod,
  accounts: Accounts
) {
  if (m.name === "new") {
    // Ctor `new` method.
    const [programSigner] = findProgramAddressSync([], programId);
    return [
      {
        pubkey: provider.wallet.publicKey,
        isWritable: false,
        isSigner: true,
      },
      {
        pubkey: programStateAddress(programId),
        isWritable: true,
        isSigner: false,
      },
      { pubkey: programSigner, isWritable: false, isSigner: false },
      {
        pubkey: SystemProgram.programId,
        isWritable: false,
        isSigner: false,
      },

      { pubkey: programId, isWritable: false, isSigner: false },
      {
        pubkey: SYSVAR_RENT_PUBKEY,
        isWritable: false,
        isSigner: false,
      },
    ];
  } else {
    validateAccounts(m.accounts, accounts);
    return [
      {
        pubkey: programStateAddress(programId),
        isWritable: true,
        isSigner: false,
      },
    ];
  }
}
