import {
  Connection,
  Signer,
  PublicKey,
  Transaction,
  TransactionSignature,
  ConfirmOptions,
  SimulatedTransactionResponse,
  Commitment,
  SendTransactionError,
  SendOptions,
  RpcResponseAndContext,
} from "@solana/web3.js";
import { bs58 } from "./utils/bytes/index.js";
import { isBrowser } from "./utils/common.js";
import {
  simulateTransaction,
  SuccessfulTxSimulationResponse,
} from "./utils/rpc.js";

export default interface Provider {
  readonly connection: Connection;

  send?(
    tx: Transaction,
    signers?: Signer[],
    opts?: SendOptions
  ): Promise<TransactionSignature>;
  sendAndConfirm?(
    tx: Transaction,
    signers?: Signer[],
    opts?: ConfirmOptions
  ): Promise<TransactionSignature>;
  sendAll?(
    txWithSigners: { tx: Transaction; signers?: Signer[] }[],
    opts?: ConfirmOptions
  ): Promise<Array<TransactionSignature>>;
  simulate?(
    tx: Transaction,
    signers?: Signer[],
    commitment?: Commitment,
    includeAccounts?: boolean | PublicKey[]
  ): Promise<SuccessfulTxSimulationResponse>;
}

/**
 * The network and wallet context used to send transactions paid for and signed
 * by the provider.
 */
export class AnchorProvider implements Provider {
  /**
   * @param connection The cluster connection where the program is deployed.
   * @param wallet     The wallet used to pay for and sign all transactions.
   * @param opts       Transaction confirmation options to use by default.
   */
  constructor(
    readonly connection: Connection,
    readonly wallet: Wallet,
    readonly opts: ConfirmOptions
  ) {}

  static defaultOptions(): ConfirmOptions {
    return {
      preflightCommitment: "processed",
      commitment: "processed",
    };
  }

  /**
   * Returns a `Provider` with a wallet read from the local filesystem.
   *
   * @param url  The network cluster url.
   * @param opts The default transaction confirmation options.
   *
   * (This api is for Node only.)
   */
  static local(url?: string, opts?: ConfirmOptions): AnchorProvider {
    if (isBrowser) {
      throw new Error(`Provider local is not available on browser.`);
    }
    opts = opts ?? AnchorProvider.defaultOptions();
    const connection = new Connection(
      url ?? "http://localhost:8899",
      opts.preflightCommitment
    );
    const NodeWallet = require("./nodewallet.js").default;
    const wallet = NodeWallet.local();
    return new AnchorProvider(connection, wallet, opts);
  }

  /**
   * Returns a `Provider` read from the `ANCHOR_PROVIDER_URL` environment
   * variable
   *
   * (This api is for Node only.)
   */
  static env(): AnchorProvider {
    if (isBrowser) {
      throw new Error(`Provider env is not available on browser.`);
    }

    const process = require("process");
    const url = process.env.ANCHOR_PROVIDER_URL;
    if (url === undefined) {
      throw new Error("ANCHOR_PROVIDER_URL is not defined");
    }
    const options = AnchorProvider.defaultOptions();
    const connection = new Connection(url, options.commitment);
    const NodeWallet = require("./nodewallet.js").default;
    const wallet = NodeWallet.local();

    return new AnchorProvider(connection, wallet, options);
  }

  /**
   * Sends the given transaction, paid for and signed by the provider's wallet.
   *
   * @param tx      The transaction to send.
   * @param signers The signers of the transaction.
   * @param opts    Transaction confirmation options.
   */
  async sendAndConfirm(
    tx: Transaction,
    signers?: Signer[],
    opts?: ConfirmOptions
  ): Promise<TransactionSignature> {
    if (opts === undefined) {
      opts = this.opts;
    }

    tx.feePayer = this.wallet.publicKey;
    tx.recentBlockhash = (
      await this.connection.getRecentBlockhash(opts.preflightCommitment)
    ).blockhash;

    tx = await this.wallet.signTransaction(tx);
    (signers ?? []).forEach((kp) => {
      tx.partialSign(kp);
    });

    const rawTx = tx.serialize();

    try {
      return await sendAndConfirmRawTransaction(this.connection, rawTx, opts);
    } catch (err) {
      // thrown if the underlying 'confirmTransaction' encounters a failed tx
      // the 'confirmTransaction' error does not return logs so we make another rpc call to get them
      if (err instanceof ConfirmError) {
        // choose the shortest available commitment for 'getTransaction'
        // (the json RPC does not support any shorter than "confirmed" for 'getTransaction')
        // because that will see the tx sent with `sendAndConfirmRawTransaction` no matter which
        // commitment `sendAndConfirmRawTransaction` used
        const failedTx = await this.connection.getTransaction(
          bs58.encode(tx.signature!),
          { commitment: "confirmed" }
        );
        if (!failedTx) {
          throw err;
        } else {
          const logs = failedTx.meta?.logMessages;
          throw !logs ? err : new SendTransactionError(err.message, logs);
        }
      } else {
        throw err;
      }
    }
  }

  /**
   * Similar to `send`, but for an array of transactions and signers.
   */
  async sendAll(
    txWithSigners: { tx: Transaction; signers?: Signer[] }[],
    opts?: ConfirmOptions
  ): Promise<Array<TransactionSignature>> {
    if (opts === undefined) {
      opts = this.opts;
    }
    const blockhash = await this.connection.getRecentBlockhash(
      opts.preflightCommitment
    );

    let txs = txWithSigners.map((r) => {
      let tx = r.tx;
      let signers = r.signers ?? [];

      tx.feePayer = this.wallet.publicKey;
      tx.recentBlockhash = blockhash.blockhash;

      signers.forEach((kp) => {
        tx.partialSign(kp);
      });

      return tx;
    });

    const signedTxs = await this.wallet.signAllTransactions(txs);

    const sigs: TransactionSignature[] = [];

    for (let k = 0; k < txs.length; k += 1) {
      const tx = signedTxs[k];
      const rawTx = tx.serialize();
      sigs.push(
        await sendAndConfirmRawTransaction(this.connection, rawTx, opts)
      );
    }

    return sigs;
  }

  /**
   * Simulates the given transaction, returning emitted logs from execution.
   *
   * @param tx      The transaction to send.
   * @param signers The signers of the transaction.
   * @param opts    Transaction confirmation options.
   */
  async simulate(
    tx: Transaction,
    signers?: Signer[],
    commitment?: Commitment,
    includeAccounts?: boolean | PublicKey[]
  ): Promise<SuccessfulTxSimulationResponse> {
    tx.feePayer = this.wallet.publicKey;
    tx.recentBlockhash = (
      await this.connection.getLatestBlockhash(
        commitment ?? this.connection.commitment
      )
    ).blockhash;

    tx = await this.wallet.signTransaction(tx);
    const result = await simulateTransaction(
      this.connection,
      tx,
      signers,
      commitment,
      includeAccounts
    );

    if (result.value.err) {
      throw new SimulateError(result.value);
    }

    return result.value;
  }
}

class SimulateError extends Error {
  constructor(
    readonly simulationResponse: SimulatedTransactionResponse,
    message?: string
  ) {
    super(message);
  }
}

export type SendTxRequest = {
  tx: Transaction;
  signers: Array<Signer | undefined>;
};

/**
 * Wallet interface for objects that can be used to sign provider transactions.
 */
export interface Wallet {
  signTransaction(tx: Transaction): Promise<Transaction>;
  signAllTransactions(txs: Transaction[]): Promise<Transaction[]>;
  publicKey: PublicKey;
}

// Copy of Connection.sendAndConfirmRawTransaction that throws
// a better error if 'confirmTransaction` returns an error status
async function sendAndConfirmRawTransaction(
  connection: Connection,
  rawTransaction: Buffer,
  options?: ConfirmOptions
): Promise<TransactionSignature> {
  const sendOptions = options && {
    skipPreflight: options.skipPreflight,
    preflightCommitment: options.preflightCommitment || options.commitment,
  };

  const signature = await connection.sendRawTransaction(
    rawTransaction,
    sendOptions
  );

  const status = (
    await connection.confirmTransaction(
      signature,
      options && options.commitment
    )
  ).value;

  if (status.err) {
    throw new ConfirmError(
      `Raw transaction ${signature} failed (${JSON.stringify(status)})`
    );
  }

  return signature;
}

class ConfirmError extends Error {
  constructor(message?: string) {
    super(message);
  }
}

/**
 * Sets the default provider on the client.
 */
export function setProvider(provider: Provider) {
  _provider = provider;
}

/**
 * Returns the default provider being used by the client.
 */
export function getProvider(): Provider {
  if (_provider === null) {
    return AnchorProvider.local();
  }
  return _provider;
}

// Global provider used as the default when a provider is not given.
let _provider: Provider | null = null;
