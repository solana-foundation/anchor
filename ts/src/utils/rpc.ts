import { Buffer } from "buffer";
import assert from "assert";
import {
  AccountInfo,
  AccountMeta,
  Connection,
  PublicKey,
  TransactionSignature,
  Transaction,
  TransactionInstruction,
  Commitment,
} from "@solana/web3.js";
import { chunks } from "../utils/common.js";
import { Address, translateAddress } from "../program/common.js";
import Provider, { getProvider } from "../provider.js";

/**
 * Sends a transaction to a program with the given accounts and instruction
 * data.
 */
export async function invoke(
  programId: Address,
  accounts?: Array<AccountMeta>,
  data?: Buffer,
  provider?: Provider
): Promise<TransactionSignature> {
  programId = translateAddress(programId);
  if (!provider) {
    provider = getProvider();
  }

  const tx = new Transaction();
  tx.add(
    new TransactionInstruction({
      programId,
      keys: accounts ?? [],
      data,
    })
  );

  return await provider.send(tx);
}

const GET_MULTIPLE_ACCOUNTS_LIMIT: number = 99;

export async function getMultipleAccounts(
  connection: Connection,
  publicKeys: PublicKey[],
  commitment?: Commitment
): Promise<
  Array<null | { publicKey: PublicKey; account: AccountInfo<Buffer> }>
> {
  if (publicKeys.length <= GET_MULTIPLE_ACCOUNTS_LIMIT) {
    return await getMultipleAccountsCore(connection, publicKeys, commitment);
  } else {
    const batches = chunks(publicKeys, GET_MULTIPLE_ACCOUNTS_LIMIT);
    const results = await Promise.all<
      Array<null | { publicKey: PublicKey; account: AccountInfo<Buffer> }>
    >(
      batches.map((batch) =>
        getMultipleAccountsCore(connection, batch, commitment)
      )
    );
    return results.flat();
  }
}

async function getMultipleAccountsCore(
  connection: Connection,
  publicKeys: PublicKey[],
  commitmentOverride?: Commitment
): Promise<
  Array<null | { publicKey: PublicKey; account: AccountInfo<Buffer> }>
> {
  const commitment = commitmentOverride ?? connection.commitment;
  const accounts = await connection.getMultipleAccountsInfo(
    publicKeys,
    commitment
  );
  return accounts.map((account, idx) => {
    if (account === null) {
      return null;
    }
    return {
      publicKey: publicKeys[idx],
      account,
    };
  });
}
