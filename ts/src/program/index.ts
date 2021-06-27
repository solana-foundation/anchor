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
  StateClient,
  SimulateNamespace,
} from "./namespace";
import { getProvider } from "../";
import { utf8 } from "../utils/bytes";
import { EventParser } from "./event";
import { Address, translateAddress } from "./common";

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
 * dynamically generated properties, also known as namespaces, that
 * map one-to-one to program methods and accounts. These namespaces generally
 *  can be used as follows:
 *
 * ## Usage
 *
 * ```javascript
 * program.<namespace>.<program-specific-method>
 * ```
 *
 * API specifics are namespace dependent. The examples used in the documentation
 * below will refer to the two counter examples found
 * [here](https://github.com/project-serum/anchor#examples).
 */
export class Program {
  /**
   * Async methods to send signed transactions to *non*-state methods on the
   * program, returning a [[TransactionSignature]].
   *
   * ## Usage
   *
   * ```javascript
   * rpc.<method>(...args, ctx);
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
   * The namespace provides handles to an [[AccountClient]] object for each
   * account in the program.
   *
   * ## Usage
   *
   * ```javascript
   * program.account.<account-client>
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
  readonly account: AccountNamespace;

  /**
   * The namespace provides functions to build [[TransactionInstruction]]
   * objects for each method of a program.
   *
   * ## Usage
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
   * The namespace provides functions to build [[Transaction]] objects for each
   * method of a program.
   *
   * ## Usage
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
   * The namespace provides functions to simulate transactions for each method
   * of a program, returning a list of deserialized events *and* raw program
   * logs.
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
   * const events = await program.simulate.increment({
   *   accounts: {
   *     counter,
   *   },
   * });
   * ```
   */
  readonly simulate: SimulateNamespace;

  /**
   * A client for the program state. Similar to the base [[Program]] client,
   * one can use this to send transactions and read accounts for the state
   * abstraction.
   */
  readonly state: StateClient;

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
  public constructor(idl: Idl, programId: Address, provider?: Provider) {
    programId = translateAddress(programId);

    // Fields.
    this._idl = idl;
    this._programId = programId;
    this._provider = provider ?? getProvider();
    this._coder = new Coder(idl);

    // Dynamic namespaces.
    const [rpc, instruction, transaction, account, simulate, state] =
      NamespaceFactory.build(idl, this._coder, programId, this._provider);
    this.rpc = rpc;
    this.instruction = instruction;
    this.transaction = transaction;
    this.account = account;
    this.simulate = simulate;
    this.state = state;
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
  public static async at(address: Address, provider?: Provider) {
    const programId = translateAddress(address);

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
  public static async fetchIdl(address: Address, provider?: Provider) {
    provider = provider ?? getProvider();
    const programId = translateAddress(address);

    const idlAddr = await idlAddress(programId);
    const accountInfo = await provider.connection.getAccountInfo(idlAddr);
    // Chop off account discriminator.
    let idlAccount = decodeIdlAccount(accountInfo.data.slice(8));
    const inflatedIdl = inflate(idlAccount.data);
    return JSON.parse(utf8.decode(inflatedIdl));
  }

  /**
   * Invokes the given callback every time the given event is emitted.
   *
   * @param eventName The PascalCase name of the event, provided by the IDL.
   * @param callback  The function to invoke whenever the event is emitted from
   *                  program logs.
   */
  public addEventListener(
    eventName: string,
    callback: (event: any, slot: number) => void
  ): number {
    const eventParser = new EventParser(this._coder, this._programId);
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
