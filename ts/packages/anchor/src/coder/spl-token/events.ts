import { EventCoder } from "../index.js";
import { Idl } from "../../idl.js";
import { Event } from "../../program/event";
import { IdlEvent } from "../../idl";

export class SplTokenEventsCoder implements EventCoder {
  constructor(_idl: Idl) {}

  decode<E extends IdlEvent = IdlEvent, T = Record<string, string>>(
    _log: string
  ): Event<E, T> | null {
    throw new Error("SPL token program does not have events");
  }
}
