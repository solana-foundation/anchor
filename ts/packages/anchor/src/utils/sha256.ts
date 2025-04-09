import { sha256 } from "@noble/hashes/sha256";

export function hash(data: string): string {
  return sha256(data).toString();
}
