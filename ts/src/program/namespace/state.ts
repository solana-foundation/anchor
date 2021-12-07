import EventEmitter from "eventemitter3";
import camelCase from "camelcase";
import {
  PublicKey,
  SystemProgram,
  Commitment,
  AccountMeta,
} from "@solana/web3.js";
import Provider, { getProvider } from "../../provider.js";
import { Idl, IdlInstruction, IdlStateMethod, IdlTypeDef } from "../../idl.js";
import Coder, { stateDiscriminator } from "../../coder/index.js";
import {
  RpcNamespace,
  InstructionNamespace,
  TransactionNamespace,
} from "./index.js";
import { Subscription, validateAccounts, parseIdlErrors } from "../common.js";
import {
  findProgramAddressSync,
  createWithSeedSync,
} from "../../utils/pubkey.js";
import { Accounts } from "../context.js";
import InstructionNamespaceFactory from "./instruction.js";
import RpcNamespaceFactory from "./rpc.js";
import TransactionNamespaceFactory from "./transaction.js";
import { IdlTypes, TypeDef } from "./types.js";

export default class StateFactory {
  public static build<IDL extends Idl>(
    idl: IDL,
    coder: Coder,
    programId: PublicKey,
    provider?: Provider
  ): StateClient<IDL> | undefined {
    if (idl.state === undefined) {
      return undefined;
    }
    return new StateClient(idl, programId, provider, coder);
  }
}

type NullableMethods<IDL extends Idl> = IDL["state"] extends undefined
  ? IdlInstruction[]
  : NonNullable<IDL["state"]>["methods"];

/**
 * A client for the program state. Similar to the base [[Program]] client,
 * one can use this to send transactions and read accounts for the state
 * abstraction.
 */
export class StateClient<IDL extends Idl> {
  /**
   * [[RpcNamespace]] for all state methods.
   */
  readonly rpc: RpcNamespace<IDL, NullableMethods<IDL>[number]>;

  /**
   * [[InstructionNamespace]] for all state methods.
   */
  readonly instruction: InstructionNamespace<IDL, NullableMethods<IDL>[number]>;

  /**
   * [[TransactionNamespace]] for all state methods.
   */
  readonly transaction: TransactionNamespace<IDL, NullableMethods<IDL>[number]>;

  /**
   * Returns the program ID owning the state.
   */
  get programId(): PublicKey {
    return this._programId;
  }
  private _programId: PublicKey;

  private _address: PublicKey;
  private _coder: Coder;
  private _idl: IDL;
  private _sub: Subscription | null;

  constructor(
    idl: IDL,
    programId: PublicKey,
    /**
     * Returns the client's wallet and network provider.
     */
    public readonly provider: Provider = getProvider(),
    /**
     * Returns the coder.
     */
    public readonly coder: Coder = new Coder(idl)
  ) {
    this._idl = idl;
    this._programId = programId;
    this._address = programStateAddress(programId);
    this._sub = null;

    // Build namespaces.
    const [instruction, transaction, rpc] = ((): [
      InstructionNamespace<IDL, NullableMethods<IDL>[number]>,
      TransactionNamespace<IDL, NullableMethods<IDL>[number]>,
      RpcNamespace<IDL, NullableMethods<IDL>[number]>
    ] => {
      let instruction: InstructionNamespace = {};
      let transaction: TransactionNamespace = {};
      let rpc: RpcNamespace = {};

      idl.state?.methods.forEach(
        <I extends NullableMethods<IDL>[number]>(m: I) => {
          // Build instruction method.
          const ixItem = InstructionNamespaceFactory.build<IDL, I>(
            m,
            (ixName, ix) => coder.instruction.encodeState(ixName, ix),
            programId
          );
          ixItem["accounts"] = (accounts) => {
            const keys = stateInstructionKeys(programId, provider, m, accounts);
            return keys.concat(
              InstructionNamespaceFactory.accountsArray(
                accounts,
                m.accounts,
                m.name
              )
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
        }
      );

      return [
        instruction as InstructionNamespace<IDL, NullableMethods<IDL>[number]>,
        transaction as TransactionNamespace<IDL, NullableMethods<IDL>[number]>,
        rpc as RpcNamespace<IDL, NullableMethods<IDL>[number]>,
      ];
    })();
    this.instruction = instruction;
    this.transaction = transaction;
    this.rpc = rpc;
  }

  /**
   * Returns the deserialized state account.
   */
  async fetch(): Promise<
    TypeDef<
      IDL["state"] extends undefined
        ? IdlTypeDef
        : NonNullable<IDL["state"]>["struct"],
      IdlTypes<IDL>
    >
  > {
    const addr = this.address();
    const accountInfo = await this.provider.connection.getAccountInfo(addr);
    if (accountInfo === null) {
      throw new Error(`Account does not exist ${addr.toString()}`);
    }
    // Assert the account discriminator is correct.
    const state = this._idl.state;
    if (!state) {
      throw new Error("State is not specified in IDL.");
    }
    const expectedDiscriminator = await stateDiscriminator(state.struct.name);
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
function stateInstructionKeys<M extends IdlStateMethod>(
  programId: PublicKey,
  provider: Provider,
  m: M,
  accounts: Accounts<M["accounts"][number]>
): AccountMeta[] {
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
