import { PublicKey } from "@solana/web3.js";
import { sha256 } from "@noble/hashes/sha256";

// Sync version of web3.PublicKey.createWithSeed.
export function createWithSeedSync(
  fromPublicKey: PublicKey,
  seed: string,
  programId: PublicKey
): PublicKey {
  const buffer = Buffer.concat([
    fromPublicKey.toBytes(),
    Buffer.from(seed) as Uint8Array,
    programId.toBytes(),
  ]);
  return new PublicKey(sha256(buffer as Uint8Array));
}
