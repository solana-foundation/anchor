import { inflate } from "pako";
import { PublicKey } from "@solana/web3.js";
import Provider from "../provider";
import { Idl, idlAddress, decodeIdlAccount } from "../idl";
import Coder from "../coder";
import NamespaceFactory, {
  Rpcs,
  Ixs,
  Txs,
  Accounts,
  State,
  Simulate,
} from "./namespace";
import { getProvider } from "../";
import { decodeUtf8 } from "../utils";
import { EventParser } from "./event";

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
   * Async functions to invoke instructions against an Anchor program.
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
   * Async functions to simulate instructions against an Anchor program.
   */
  readonly simulate: Simulate;

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

    // Build the dynamic namespaces.
    const [rpcs, ixs, txs, accounts, state, simulate] = NamespaceFactory.build(
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
    this.simulate = simulate;
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

  /**
   * Invokes the given callback everytime the given event is emitted.
   */
  public addEventListener(
    eventName: string,
    callback: (event: any, slot: number) => void
  ): number {
    const eventParser = new EventParser(this.coder, this.programId, this.idl);
    return this.provider.connection.onLogs(this.programId, (logs, ctx) => {
      if (logs.err) {
        console.error(logs);
        return;
      }
      eventParser.parseLogs(logs.logs, (event) => {
        if (event.name === eventName) {
          callback(event.data, ctx.slot);
        }
      });
    });
  }

  public async removeEventListener(listener: number): Promise<void> {
    return this.provider.connection.removeOnLogsListener(listener);
  }
}
