import { inflate } from "pako";
import { PublicKey } from "@solana/web3.js";
import Provider from "../provider";
import { Idl, idlAddress, decodeIdlAccount } from "../idl";
import Coder from "../coder";
import NamespaceFactory, {
  RpcNamespace,
  InstructionNamespace,
  TransactionNamespace,
  AccountNamespace,
  StateNamespace,
  SimulateNamespace,
} from "./namespace";
import { getProvider } from "../";
import { decodeUtf8 } from "../utils";
import { EventParser } from "./event";

/**
 * ## Program
 *
 * Program provides the IDL deserialized client representation of an Anchor
 * program.
 *
 * This API is the one stop shop for all things related to communicating with
 * on-chain programs. Among other things, one can send transactions, fetch
 * deserialized accounts, decode instruction data, subscribe to account
 * changes, and listen to events.
 *
 * In addition to field accessors and methods, the object provides a set of
 * dynamically generated properties (internally referred to as namespaces) that
 * map one-to-one to program instructions and accounts. These namespaces
 * generally can be used as follows:
 *
 * ```javascript
 * program.<namespace>.<program-specific-field>
 * ```
 *
 * API specifics are namespace dependent. The examples used in the documentation
 * below will refer to the two counter examples found
 * [here](https://project-serum.github.io/anchor/ts/#examples).
 */
export class Program {
  /**
   * Async methods to send signed transactions invoking *non*-state methods
   * on an Anchor program.
   *
   * ## rpc
   *
   * ```javascript
   * program.rpc.<method>(...args, ctx);
   * ```
   *
   * ## Parameters
   *
   * 1. `args` - The positional arguments for the program. The type and number
   *    of these arguments depend on the program being used.
   * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
   *    Always the last parameter in the method call.
   *
   * ## Example
   *
   * To send a transaction invoking the `increment` method above,
   *
   * ```javascript
   * const txSignature = await program.rpc.increment({
   *   accounts: {
   *     counter,
	 *     authority,
   *   },
   * });
   * ```
   */
  readonly rpc: RpcNamespace;

  /**
   * Async functions to fetch deserialized program accounts from a cluster.
   *
   * ## account
   *
   * ```javascript
   * program.account.<account>(publicKey);
   * ```
   *
   * ## Parameters
   *
   * 1. `publicKey` - The [[PublicKey]] of the account.
   *
   * ## Example
   *
   * To fetch a `Counter` object from the above example,
   *
   * ```javascript
   * const counter = await program.account.counter(publicKey);
   * ```
   */
  readonly account: AccountNamespace;

  /**
   * Functions to build [[TransactionInstruction]] objects for program methods.
   *
   * ## instruction
   *
   * ```javascript
   * program.instruction.<method>(...args, ctx);
   * ```
   *
   * ## Parameters
   *
   * 1. `args` - The positional arguments for the program. The type and number
   *    of these arguments depend on the program being used.
   * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
   *    Always the last parameter in the method call.
   *
   * ## Example
   *
   * To create an instruction for the `increment` method above,
   *
   * ```javascript
   * const tx = await program.instruction.increment({
   *   accounts: {
   *     counter,
   *   },
   * });
   * ```
   */
  readonly instruction: InstructionNamespace;

  /**
   * Functions to build [[Transaction]] objects.
   *
   * ## transaction
   *
   * ```javascript
   * program.transaction.<method>(...args, ctx);
   * ```
   *
   * ## Parameters
   *
   * 1. `args` - The positional arguments for the program. The type and number
   *    of these arguments depend on the program being used.
   * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
   *    Always the last parameter in the method call.
   *
   * ## Example
   *
   * To create an instruction for the `increment` method above,
   *
   * ```javascript
   * const tx = await program.transaction.increment({
   *   accounts: {
   *     counter,
   *   },
   * });
   * ```
   */
  readonly transaction: TransactionNamespace;

  /**
   * Async functions to simulate instructions against an Anchor program,
   * returning a list of deserialized events *and* raw program logs.
   *
   * One can use this to read data calculated from a program on chain, by
   * emitting an event in the program and reading the emitted event client side
   * via the `simulate` namespace.
   *
   * ## simulate
   *
   * ```javascript
   * program.simulate.<method>(...args, ctx);
   * ```
   *
   * ## Parameters
   *
   * 1. `args` - The positional arguments for the program. The type and number
   *    of these arguments depend on the program being used.
   * 2. `ctx`  - [[Context]] non-argument parameters to pass to the method.
   *    Always the last parameter in the method call.
   *
   * ## Example
   *
   * To simulate the `increment` method above,
   *
   * ```javascript
   * const tx = await program.simulate.increment({
   *   accounts: {
   *     counter,
   *   },
   * });
   * ```
   */
  readonly simulate: SimulateNamespace;

  /**
   * Object with state account accessors and rpcs.
   */
  readonly state: StateNamespace;

  /**
   * Address of the program.
   */
  public get programId(): PublicKey {
    return this._programId;
  }
  private _programId: PublicKey;

  /**
   * IDL defining the program's interface.
   */
  public get idl(): Idl {
    return this._idl;
  }
  private _idl: Idl;

  /**
   * Coder for serializing requests.
   */
  public get coder(): Coder {
    return this._coder;
  }
  private _coder: Coder;

  /**
   * Wallet and network provider.
   */
  public get provider(): Provider {
    return this._provider;
  }
  private _provider: Provider;

  /**
   * @param idl       The interface definition.
   * @param programId The on-chain address of the program.
   * @param provider  The network and wallet context to use. If not provided
   *                  then uses [[getProvider]].
   */
  public constructor(idl: Idl, programId: PublicKey, provider?: Provider) {
    // Fields.
    this._idl = idl;
    this._programId = programId;
    this._provider = provider ?? getProvider();
    this._coder = new Coder(idl);

    // Dynamic namespaces.
    const [
      rpc,
      instruction,
      transaction,
      account,
      state,
      simulate,
    ] = NamespaceFactory.build(idl, this._coder, programId, this._provider);
    this.rpc = rpc;
    this.instruction = instruction;
    this.transaction = transaction;
    this.account = account;
    this.state = state;
    this.simulate = simulate;
  }

  /**
   * Generates a Program client by fetching the IDL from the network.
   *
   * In order to use this method, an IDL must have been previously initialized
   * via the anchor CLI's `anchor idl init` command.
   *
   * @param programId The on-chain address of the program.
   * @param provider  The network and wallet context.
   */
  public static async at(programId: PublicKey, provider?: Provider) {
    const idl = await Program.fetchIdl(programId, provider);
    return new Program(idl, programId, provider);
  }

  /**
   * Fetches an idl from the blockchain.
   *
   * In order to use this method, an IDL must have been previously initialized
   * via the anchor CLI's `anchor idl init` command.
   *
   * @param programId The on-chain address of the program.
   * @param provider  The network and wallet context.
   */
  public static async fetchIdl(programId: PublicKey, provider?: Provider) {
    provider = provider ?? getProvider();
    const address = await idlAddress(programId);
    const accountInfo = await provider.connection.getAccountInfo(address);
    // Chop off account discriminator.
    let idlAccount = decodeIdlAccount(accountInfo.data.slice(8));
    const inflatedIdl = inflate(idlAccount.data);
    return JSON.parse(decodeUtf8(inflatedIdl));
  }

  /**
   * Invokes the given callback everytime the given event is emitted.
   *
   * @param eventName The PascalCase name of the event, provided by the IDL.
   * @param callback  The function to invoke whenever the event is emitted from
   *                  program logs.
   */
  public addEventListener(
    eventName: string,
    callback: (event: any, slot: number) => void
  ): number {
    const eventParser = new EventParser(
      this._coder,
      this._programId,
      this._idl
    );
    return this._provider.connection.onLogs(this._programId, (logs, ctx) => {
      if (logs.err) {
        console.error(logs);
        return;
      }
      eventParser.parseLogs(logs.logs, (event) => {
        if (event.name === eventName) {
          callback(event.data, ctx.slot);
        }
      });
    });
  }

  /**
   * Unsubscribes from the given event listener.
   */
  public async removeEventListener(listener: number): Promise<void> {
    return this._provider.connection.removeOnLogsListener(listener);
  }
}
