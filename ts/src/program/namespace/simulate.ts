import { PublicKey } from "@solana/web3.js";
import Provider from "../../provider";
import { IdlInstruction } from "../../idl";
import { translateError } from "../common";
import { splitArgsAndCtx } from "../context";
import { TxFn } from "./transaction";
import { EventParser } from "../event";
import Coder from "../../coder";
import { Idl } from "../../idl";

/**
 * Dynamically generated simualte namespace.
 */
export interface Simulate {
  [key: string]: SimulateFn;
}

/**
 * RpcFn is a single rpc method generated from an IDL.
 */
export type SimulateFn = (...args: any[]) => Promise<SimulateResponse>;

type SimulateResponse = {
  events: Event[];
  raw: string[];
};

export default class SimulateNamespace {
  // Builds the rpc namespace.
  public static build(
    idlIx: IdlInstruction,
    txFn: TxFn,
    idlErrors: Map<number, string>,
    provider: Provider,
    coder: Coder,
    programId: PublicKey,
    idl: Idl
  ): SimulateFn {
    const simulate = async (...args: any[]): Promise<SimulateResponse> => {
      const tx = txFn(...args);
      const [_, ctx] = splitArgsAndCtx(idlIx, [...args]);
      let resp = undefined;
      try {
        resp = await provider.simulate(tx, ctx.signers, ctx.options);
      } catch (err) {
        console.log("Translating error", err);
        let translatedErr = translateError(idlErrors, err);
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
        let parser = new EventParser(coder, programId, idl);
        parser.parseLogs(logs, (event) => {
          events.push(event);
        });
      }
      return { events, raw: logs };
    };

    return simulate;
  }
}
