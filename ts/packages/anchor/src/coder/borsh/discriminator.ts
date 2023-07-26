import { sha256 } from "@noble/hashes/sha256";

/**
 * Number of bytes in anchor discriminators
 */
export const DISCRIMINATOR_SIZE = 8;

export function discriminator(preimage: string): Buffer {
  return Buffer.from(sha256(preimage).slice(0, DISCRIMINATOR_SIZE));
}
