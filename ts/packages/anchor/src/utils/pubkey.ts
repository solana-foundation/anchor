import { Buffer } from "buffer";
import { PublicKey } from "@solana/web3.js";
import { Address, translateAddress } from "../program/common.js";
import { sha256 } from "@noble/hashes/sha256";

// Sync version of web3.PublicKey.createWithSeed.
export function createWithSeedSync(
  fromPublicKey: PublicKey,
  seed: string,
  programId: PublicKey
): PublicKey {
  const buffer = Buffer.concat([
    fromPublicKey.toBuffer(),
    Buffer.from(seed),
    programId.toBuffer(),
  ]);
  return new PublicKey(sha256(buffer));
}

export function associated(
  programId: Address,
  ...args: Array<Address | Buffer>
): PublicKey {
  let seeds = [Buffer.from([97, 110, 99, 104, 111, 114])]; // b"anchor".
  args.forEach((arg) => {
    seeds.push(arg instanceof Buffer ? arg : translateAddress(arg).toBuffer());
  });
  const [assoc] = PublicKey.findProgramAddressSync(
    seeds,
    translateAddress(programId)
  );
  return assoc;
}
