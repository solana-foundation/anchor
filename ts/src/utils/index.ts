import { sha256 } from "crypto-hash";
import * as bs58 from "bs58";
import * as rpc from "./rpc";
import * as publicKey from "./pubkey";

export function decodeUtf8(array: Uint8Array): string {
  const decoder =
    typeof TextDecoder === "undefined"
      ? new (require("util").TextDecoder)("utf-8") // Node.
      : new TextDecoder("utf-8"); // Browser.
  return decoder.decode(array);
}

export default { sha256, bs58, rpc, publicKey };
