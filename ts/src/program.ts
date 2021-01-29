import { PublicKey } from "@solana/web3.js";
import { inflate } from "pako";
import Provider from "./provider";
import { RpcFactory } from "./rpc";
import { Idl, idlAddress, decodeIdlAccount } from "./idl";
import Coder from "./coder";
import { Rpcs, Ixs, Txs, Accounts, State } from "./rpc";
import { getProvider } from "./";

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

  /**
   * Object with state account accessors and rpcs.
   */
  readonly state: State;

  /**
   * Wallet and network provider.
   */
  readonly provider: Provider;

  public constructor(idl: Idl, programId: PublicKey, provider?: Provider) {
    this.idl = idl;
    this.programId = programId;
    this.provider = provider ?? getProvider();

    // Build the serializer.
    const coder = new Coder(idl);

    // Build the dynamic RPC functions.
    const [rpcs, ixs, txs, accounts, state] = RpcFactory.build(
      idl,
      coder,
      programId,
      this.provider
    );
    this.rpc = rpcs;
    this.instruction = ixs;
    this.transaction = txs;
    this.account = accounts;
    this.coder = coder;
    this.state = state;
  }

  /**
   * Generates a Program client by fetching the IDL from chain.
   */
  public static async at(programId: PublicKey, provider?: Provider) {
    const idl = await Program.fetchIdl(programId, provider);
    return new Program(idl, programId, provider);
  }

  /**
   * Fetches an idl from the blockchain.
   */
  public static async fetchIdl(programId: PublicKey, provider?: Provider) {
    provider = provider ?? getProvider();
    const address = await idlAddress(programId);
    const accountInfo = await provider.connection.getAccountInfo(address);
    // Chop off account discriminator.
    let idlAccount = decodeIdlAccount(accountInfo.data.slice(8));
    const inflatedIdl = inflate(idlAccount.data);
    return JSON.parse(decodeUtf8(inflatedIdl));
  }
}

function decodeUtf8(array: Uint8Array): string {
  const decoder =
    typeof TextDecoder === "undefined"
      ? new (require("util").TextDecoder)("utf-8") // Node.
      : new TextDecoder("utf-8"); // Browser.
  return decoder.decode(array);
}
