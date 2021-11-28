import { Buffer } from "buffer";
import BN from "bn.js";
import { sha256 as sha256Sync } from "js-sha256";
import { PublicKey } from "@solana/web3.js";
import { Address, translateAddress } from "../program/common.js";

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
  const hash = sha256Sync.digest(buffer);
  return new PublicKey(Buffer.from(hash));
}

// Sync version of web3.PublicKey.createProgramAddress.
export function createProgramAddressSync(
  seeds: Array<Buffer | Uint8Array>,
  programId: PublicKey
): PublicKey {
  const MAX_SEED_LENGTH = 32;

  let buffer = Buffer.alloc(0);
  seeds.forEach(function (seed) {
    if (seed.length > MAX_SEED_LENGTH) {
      throw new TypeError(`Max seed length exceeded`);
    }
    buffer = Buffer.concat([buffer, toBuffer(seed)]);
  });
  buffer = Buffer.concat([
    buffer,
    programId.toBuffer(),
    Buffer.from("ProgramDerivedAddress"),
  ]);
  let hash = sha256Sync(new Uint8Array(buffer));
  let publicKeyBytes = new BN(hash, 16).toArray(undefined, 32);
  if (PublicKey.isOnCurve(new Uint8Array(publicKeyBytes))) {
    throw new Error(`Invalid seeds, address must fall off the curve`);
  }
  return new PublicKey(publicKeyBytes);
}

// Sync version of web3.PublicKey.findProgramAddress.
export function findProgramAddressSync(
  seeds: Array<Buffer | Uint8Array>,
  programId: PublicKey
): [PublicKey, number] {
  let nonce = 255;
  let address: PublicKey | undefined;
  while (nonce != 0) {
    try {
      const seedsWithNonce = seeds.concat(Buffer.from([nonce]));
      address = createProgramAddressSync(seedsWithNonce, programId);
    } catch (err) {
      if (err instanceof TypeError) {
        throw err;
      }
      nonce--;
      continue;
    }
    return [address, nonce];
  }
  throw new Error(`Unable to find a viable program address nonce`);
}

const toBuffer = (arr: Buffer | Uint8Array | Array<number>): Buffer => {
  if (arr instanceof Buffer) {
    return arr;
  } else if (arr instanceof Uint8Array) {
    return Buffer.from(arr.buffer, arr.byteOffset, arr.byteLength);
  } else {
    return Buffer.from(arr);
  }
};

export async function associated(
  programId: Address,
  ...args: Array<PublicKey | Buffer>
): Promise<PublicKey> {
  let seeds = [Buffer.from([97, 110, 99, 104, 111, 114])]; // b"anchor".
  args.forEach((arg) => {
    seeds.push(
      // @ts-ignore
      arg.buffer !== undefined ? arg : translateAddress(arg).toBuffer()
    );
  });
  const [assoc] = await PublicKey.findProgramAddress(
    seeds,
    translateAddress(programId)
  );
  return assoc;
}
