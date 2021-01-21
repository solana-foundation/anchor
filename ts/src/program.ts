import { PublicKey } from "@solana/web3.js";
import { RpcFactory } from "./rpc";
import { Idl } from "./idl";
import Coder from "./coder";
import { Rpcs, Ixs, Txs, Accounts } from "./rpc";

/**
 * Program is the IDL deserialized representation of a Solana program.
 */
export class Program {
  /**
   * Address of the program.
   */
  readonly programId: PublicKey;

  /**
   * IDL describing this program's interface.
   */
  readonly idl: Idl;

  /**
   * Async functions to invoke instructions against a Solana priogram running
   * on a cluster.
   */
  readonly rpc: Rpcs;

  /**
   * Async functions to fetch deserialized program accounts from a cluster.
   */
  readonly account: Accounts;

  /**
   * Functions to build `TransactionInstruction` objects.
   */
  readonly instruction: Ixs;

  /**
   * Functions to build `Transaction` objects.
   */
  readonly transaction: Txs;

  /**
   * Coder for serializing rpc requests.
   */
  readonly coder: Coder;

  public constructor(idl: Idl, programId: PublicKey) {
    this.idl = idl;
    this.programId = programId;

    // Build the serializer.
    const coder = new Coder(idl);

    // Build the dynamic RPC functions.
    const [rpcs, ixs, txs, accounts] = RpcFactory.build(idl, coder, programId);
    this.rpc = rpcs;
    this.instruction = ixs;
    this.transaction = txs;
    this.account = accounts;
    this.coder = coder;
  }
}
