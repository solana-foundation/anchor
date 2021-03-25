import { PublicKey } from "@solana/web3.js";
import { inflate } from "pako";
import Provider from "./provider";
import { RpcFactory } from "./rpc";
import { Idl, idlAddress, decodeIdlAccount } from "./idl";
import Coder, { eventDiscriminator } from "./coder";
import { Rpcs, Ixs, Txs, Accounts, State } from "./rpc";
import { getProvider } from "./";
import * as base64 from "base64-js";
import * as assert from "assert";

/**
 * Program is the IDL deserialized representation of a Solana program.
 */
export class Program {
  /**
   * Address of the program.
   */
  readonly programId: PublicKey;

  /**
   * IDL describing this program's interface.
   */
  readonly idl: Idl;

  /**
   * Async functions to invoke instructions against a Solana priogram running
   * on a cluster.
   */
  readonly rpc: Rpcs;

  /**
   * Async functions to fetch deserialized program accounts from a cluster.
   */
  readonly account: Accounts;

  /**
   * Functions to build `TransactionInstruction` objects.
   */
  readonly instruction: Ixs;

  /**
   * Functions to build `Transaction` objects.
   */
  readonly transaction: Txs;

  /**
   * Coder for serializing rpc requests.
   */
  readonly coder: Coder;

  /**
   * Object with state account accessors and rpcs.
   */
  readonly state: State;

  /**
   * Wallet and network provider.
   */
  readonly provider: Provider;

  public constructor(idl: Idl, programId: PublicKey, provider?: Provider) {
    this.idl = idl;
    this.programId = programId;
    this.provider = provider ?? getProvider();

    // Build the serializer.
    const coder = new Coder(idl);

    // Build the dynamic RPC functions.
    const [rpcs, ixs, txs, accounts, state] = RpcFactory.build(
      idl,
      coder,
      programId,
      this.provider
    );
    this.rpc = rpcs;
    this.instruction = ixs;
    this.transaction = txs;
    this.account = accounts;
    this.coder = coder;
    this.state = state;
  }

  /**
   * Generates a Program client by fetching the IDL from chain.
   */
  public static async at(programId: PublicKey, provider?: Provider) {
    const idl = await Program.fetchIdl(programId, provider);
    return new Program(idl, programId, provider);
  }

  /**
   * Fetches an idl from the blockchain.
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
   */
  public addEventListener<T>(
    eventName: string,
    callback: (event: T, slot: number) => void
  ): Promise<void> {
    // Values shared across log handlers.
    const thisProgramStr = this.programId.toString();
    const discriminator = eventDiscriminator(eventName);
    const logStartIndex = "Program log: ".length;

    // Handles logs when the current program being executing is *not* this.
    const handleSystemLog = (log: string): [string | null, boolean] => {
      // System component.
      const logStart = log.split(":")[0];
      // Recursive call.
      if (logStart.startsWith(`Program ${this.programId.toString()} invoke`)) {
        return [this.programId.toString(), false];
      }
      // Cpi call.
      else if (logStart.includes("invoke")) {
        return ["cpi", false]; // Any string will do.
      } else {
        // Did the program finish executing?
        if (logStart.match(/^Program (.*) consumed .*$/g) !== null) {
          return [null, true];
        }
        return [null, false];
      }
    };

    // Handles logs from *this* program.
    const handleProgramLog = (
      log: string
    ): [T | null, string | null, boolean] => {
      // This is a `msg!` log.
      if (log.startsWith("Program log:")) {
        const logStr = log.slice(logStartIndex);
        const logArr = Buffer.from(base64.toByteArray(logStr));
        const disc = logArr.slice(0, 8);
        // Only deserialize if the discriminator implies a proper event.
        let event = null;
        if (disc.equals(discriminator)) {
          event = this.coder.events.decode(eventName, logArr.slice(8));
        }
        return [event, null, false];
      }
      // System log.
      else {
        return [null, ...handleSystemLog(log)];
      }
    };

    // Main log handler. Returns a three element array of the event, the
    // next program that was invoked for CPI, and a boolean indicating if
    // a program has completed execution (and thus should be popped off the
    // execution stack).
    const handleLog = (
      execution: ExecutionContext,
      log: string
    ): [T | null, string | null, boolean] => {
      // Executing program is this program.
      if (execution.program() === thisProgramStr) {
        return handleProgramLog(log);
      }
      // Executing program is not this program.
      else {
        return [null, ...handleSystemLog(log)];
      }
    };

    // Each log given, represents an array of messages emitted by
    // a single transaction, which can execute many different programs across
    // CPI boundaries. However, the subscription is only interested in the
    // events emitted by *this* program. In achieving this, we keep track of the
    // program execution context by parsing each log and looking for a CPI
    // `invoke` call. If one exists, we know a new program is executing. So we
    // push the programId onto a stack and switch the program context. This
    // allows us to track, for a given log, which program was executing during
    // its emission, thereby allowing us to know if a given log event was
    // emitted by *this* program. If it was, then we parse the raw string and
    // emit the event if the string matches the event being subscribed to.
    //
    // @ts-ignore
    return this.provider.connection.onLogs(this.programId, (logs, ctx) => {
      if (logs.err) {
        console.error(logs);
        return;
      }

      const logScanner = new LogScanner(logs.logs);
      const execution = new ExecutionContext(logScanner.next() as string);

      let log = logScanner.next();
      while (log !== null) {
        let [event, newProgram, didPop] = handleLog(execution, log);
        if (event) {
          callback(event, ctx.slot);
        }
        if (newProgram) {
          execution.push(newProgram);
        }
        if (didPop) {
          execution.pop();
        }
        log = logScanner.next();
      }
    });
  }

  public async removeEventListener(listener: number): Promise<void> {
    // @ts-ignore
    return this.provider.connection.removeOnLogsListener(listener);
  }
}

// Stack frame execution context, allowing one to track what program is
// executing for a given log.
class ExecutionContext {
  stack: string[];

  constructor(log: string) {
    // Assumes the first log in every transaction is an `invoke` log from the
    // runtime.
    const program = /^Program (.*) invoke.*$/g.exec(log)[1];
    this.stack = [program];
  }

  program(): string {
    assert.ok(this.stack.length > 0);
    return this.stack[this.stack.length - 1];
  }

  push(newProgram: string) {
    this.stack.push(newProgram);
  }

  pop() {
    assert.ok(this.stack.length > 0);
    this.stack.pop();
  }
}

class LogScanner {
  constructor(public logs: string[]) {}

  next(): string | null {
    if (this.logs.length === 0) {
      return null;
    }
    let l = this.logs[0];
    this.logs = this.logs.slice(1);
    return l;
  }
}

function decodeUtf8(array: Uint8Array): string {
  const decoder =
    typeof TextDecoder === "undefined"
      ? new (require("util").TextDecoder)("utf-8") // Node.
      : new TextDecoder("utf-8"); // Browser.
  return decoder.decode(array);
}
