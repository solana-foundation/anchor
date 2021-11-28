import { Buffer } from "buffer";
import * as base64 from "base64-js";

export function encode(data: Buffer): string {
  return base64.fromByteArray(data);
}

export function decode(data: string): Buffer {
  return Buffer.from(base64.toByteArray(data));
}
