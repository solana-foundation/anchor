import {
  PublicKey,
  RpcResponseAndContext,
  SimulatedTransactionResponse,
} from "@solana/web3.js";
import Provider from "../../provider.js";
import { splitArgsAndCtx } from "../context.js";
import { TransactionFn } from "./transaction.js";
import { EventParser, Event } from "../event.js";
import { Coder } from "../../coder/index.js";
import { Idl, IdlEvent } from "../../idl.js";
import { ProgramError } from "../../error.js";
import * as features from "../../utils/features.js";
import {
  AllInstructions,
  IdlTypes,
  InstructionContextFn,
  MakeInstructionsNamespace,
} from "./types";

export default class SimulateFactory {
  public static build<IDL extends Idl, I extends AllInstructions<IDL>>(
    idlIx: AllInstructions<IDL>,
    txFn: TransactionFn<IDL>,
    idlErrors: Map<number, string>,
    provider: Provider,
    coder: Coder,
    programId: PublicKey,
    idl: IDL
  ): SimulateFn<IDL, I> {
    const simulate: SimulateFn<IDL> = async (...args) => {
      const tx = txFn(...args);
      const [, ctx] = splitArgsAndCtx(idlIx, [...args]);
      let resp:
        | RpcResponseAndContext<SimulatedTransactionResponse>
        | undefined = undefined;
      try {
        resp = await provider!.simulate(tx, ctx.signers, ctx.options);
      } catch (err) {
        if (features.isSet("debug-logs")) {
          console.log("Translating error:", err);
        }

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

      const events: Event<IdlEvent, IdlTypes<IDL>>[] = [];
      if (idl.events) {
        let parser = new EventParser(programId, coder);
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
export type SimulateNamespace<
  IDL extends Idl = Idl,
  I extends AllInstructions<IDL> = AllInstructions<IDL>
> = MakeInstructionsNamespace<
  IDL,
  I,
  Promise<SimulateResponse<NullableEvents<IDL>, IdlTypes<IDL>>>
>;

type NullableEvents<IDL extends Idl> = IDL["events"] extends undefined
  ? IdlEvent
  : NonNullable<IDL["events"]>[number];

/**
 * SimulateFn is a single method generated from an IDL. It simulates a method
 * against a cluster configured by the provider, returning a list of all the
 * events and raw logs that were emitted during the execution of the
 * method.
 */
export type SimulateFn<
  IDL extends Idl = Idl,
  I extends AllInstructions<IDL> = AllInstructions<IDL>
> = InstructionContextFn<
  IDL,
  I,
  Promise<SimulateResponse<NullableEvents<IDL>, IdlTypes<IDL>>>
>;

export type SimulateResponse<E extends IdlEvent, Defined> = {
  events: readonly Event<E, Defined>[];
  raw: readonly string[];
};
