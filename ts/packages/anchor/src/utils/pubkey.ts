import { PublicKey } from "@solana/web3.js";
import { sha256 } from "@noble/hashes/sha256";

// Sync version of web3.PublicKey.createWithSeed.
export function createWithSeedSync(
  fromPublicKey: PublicKey,
  seed: string,
  programId: PublicKey
): PublicKey {
  const fromKey = fromPublicKey.toBytes();
  const seedBytes = new TextEncoder().encode(seed);
  const program = programId.toBytes();
  const data = new Uint8Array(
    fromKey.length + seedBytes.length + program.length
  );
  data.set(fromKey, 0);
  data.set(seedBytes, fromKey.length);
  data.set(program, fromKey.length + seedBytes.length);
  return new PublicKey(sha256(data));
}
