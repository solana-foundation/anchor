import { PublicKey } from "@solana/web3.js";
import Provider from "../../provider";
import { IdlInstruction } from "../../idl";
import { splitArgsAndCtx } from "../context";
import { TransactionFn } from "./transaction";
import { EventParser, Event } from "../event";
import Coder from "../../coder";
import { Idl } from "../../idl";
import { ProgramError } from "../../error";

export default class SimulateFactory {
  public static build(
    idlIx: IdlInstruction,
    txFn: TransactionFn,
    idlErrors: Map<number, string>,
    provider: Provider,
    coder: Coder,
    programId: PublicKey,
    idl: Idl
  ): SimulateFn {
    const simulate = async (...args: any[]): Promise<SimulateResponse> => {
      const tx = txFn(...args);
      const [, ctx] = splitArgsAndCtx(idlIx, [...args]);
      let resp = undefined;
      try {
        resp = await provider.simulate(tx, ctx.signers, ctx.options);
      } catch (err) {
        console.log("Translating error", err);
        let translatedErr = ProgramError.parse(err, idlErrors);
        if (translatedErr === null) {
          throw err;
        }
        throw translatedErr;
      }
      if (resp === undefined) {
        throw new Error("Unable to simulate transaction");
      }
      if (resp.value.err) {
        throw new Error(`Simulate error: ${resp.value.err.toString()}`);
      }
      const logs = resp.value.logs;
      if (!logs) {
        throw new Error("Simulated logs not found");
      }

      const events = [];
      if (idl.events) {
        let parser = new EventParser(coder, programId);
        parser.parseLogs(logs, (event) => {
          events.push(event);
        });
      }
      return { events, raw: logs };
    };

    return simulate;
  }
}

/**
 * The namespace provides functions to simulate transactions for each method
 * of a program, returning a list of deserialized events *and* raw program
 * logs.
 *
 * One can use this to read data calculated from a program on chain, by
 * emitting an event in the program and reading the emitted event client side
 * via the `simulate` namespace.
 *
 * ## Usage
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
export interface SimulateNamespace {
  [key: string]: SimulateFn;
}

/**
 * RpcFn is a single method generated from an IDL. It simulates a method
 * against a cluster configured by the provider, returning a list of all the
 * events and raw logs that were emitted during the execution of the
 * method.
 */
export type SimulateFn = (...args: any[]) => Promise<SimulateResponse>;

type SimulateResponse = {
  events: Event[];
  raw: string[];
};
