import {
  ConfirmOptions,
  AccountMeta,
  Signer,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
  PublicKey,
} from "@solana/web3.js";
import { SimulateResponse } from "./simulate.js";
import { TransactionFn } from "./transaction.js";
import { Idl } from "../../idl.js";
import { AllInstructions, MethodsFn, MakeMethodsNamespace } from "./types.js";
import { InstructionFn } from "./instruction.js";
import { RpcFn } from "./rpc.js";
import { SimulateFn } from "./simulate.js";
import Provider from "../../provider.js";
import { AccountNamespace } from "./account.js";
import { AccountsResolver } from "../accounts-resolver.js";

export type MethodsNamespace<
  IDL extends Idl = Idl,
  I extends AllInstructions<IDL> = AllInstructions<IDL>
> = MakeMethodsNamespace<IDL, I>;

export class MethodsBuilderFactory {
  public static build<IDL extends Idl, I extends AllInstructions<IDL>>(
    provider: Provider,
    programId: PublicKey,
    idlIx: AllInstructions<IDL>,
    ixFn: InstructionFn<IDL>,
    txFn: TransactionFn<IDL>,
    rpcFn: RpcFn<IDL>,
    simulateFn: SimulateFn<IDL>,
    accountNamespace: AccountNamespace<IDL>
  ): MethodsFn<IDL, I, any> {
    const request: MethodsFn<IDL, I, any> = (...args) => {
      return new MethodsBuilder(
        args,
        ixFn,
        txFn,
        rpcFn,
        simulateFn,
        provider,
        programId,
        idlIx,
        accountNamespace
      );
    };
    return request;
  }
}

export class MethodsBuilder<IDL extends Idl, I extends AllInstructions<IDL>> {
  readonly _accounts: { [name: string]: PublicKey } = {};
  private _remainingAccounts: Array<AccountMeta> = [];
  private _signers: Array<Signer> = [];
  private _preInstructions: Array<TransactionInstruction> = [];
  private _postInstructions: Array<TransactionInstruction> = [];
  private _accountsResolver: AccountsResolver<IDL, I>;

  constructor(
    private _args: Array<any>,
    private _ixFn: InstructionFn<IDL>,
    private _txFn: TransactionFn<IDL>,
    private _rpcFn: RpcFn<IDL>,
    private _simulateFn: SimulateFn<IDL>,
    _provider: Provider,
    _programId: PublicKey,
    _idlIx: AllInstructions<IDL>,
    _accountNamespace: AccountNamespace<IDL>
  ) {
    this._accountsResolver = new AccountsResolver(
      _args,
      this._accounts,
      _provider,
      _programId,
      _idlIx,
      _accountNamespace
    );
  }

  // TODO: don't use any.
  public accounts(accounts: any): MethodsBuilder<IDL, I> {
    Object.assign(this._accounts, accounts);
    return this;
  }

  public signers(signers: Array<Signer>): MethodsBuilder<IDL, I> {
    this._signers = this._signers.concat(signers);
    return this;
  }

  public remainingAccounts(
    accounts: Array<AccountMeta>
  ): MethodsBuilder<IDL, I> {
    this._remainingAccounts = this._remainingAccounts.concat(accounts);
    return this;
  }

  public preInstructions(
    ixs: Array<TransactionInstruction>
  ): MethodsBuilder<IDL, I> {
    this._preInstructions = this._preInstructions.concat(ixs);
    return this;
  }

  public postInstructions(
    ixs: Array<TransactionInstruction>
  ): MethodsBuilder<IDL, I> {
    this._postInstructions = this._postInstructions.concat(ixs);
    return this;
  }

  public async rpc(options: ConfirmOptions): Promise<TransactionSignature> {
    await this._accountsResolver.resolve();
    // @ts-ignore
    return this._rpcFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
      options: options,
    });
  }

  public async simulate(
    options: ConfirmOptions
  ): Promise<SimulateResponse<any, any>> {
    await this._accountsResolver.resolve();
    // @ts-ignore
    return this._simulateFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
      options: options,
    });
  }

  public async instruction(): Promise<TransactionInstruction> {
    await this._accountsResolver.resolve();
    // @ts-ignore
    return this._ixFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
    });
  }

  public async transaction(): Promise<Transaction> {
    await this._accountsResolver.resolve();
    // @ts-ignore
    return this._txFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
    });
  }
}
