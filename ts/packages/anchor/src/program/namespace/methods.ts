import {
  AccountMeta,
  ConfirmOptions,
  PublicKey,
  Signer,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import { Idl, IdlAccountItem, IdlAccounts, IdlTypeDef } from "../../idl.js";
import Provider from "../../provider.js";
import {
  AccountsGeneric,
  AccountsResolver,
  CustomAccountResolver,
} from "../accounts-resolver.js";
import { Address, translateAddress } from "../common.js";
import { Accounts } from "../context.js";
import { AccountNamespace } from "./account.js";
import { InstructionFn } from "./instruction.js";
import { RpcFn } from "./rpc.js";
import { SimulateFn, SimulateResponse } from "./simulate.js";
import { TransactionFn } from "./transaction.js";
import {
  AllInstructions,
  InstructionAccountAddresses,
  MakeMethodsNamespace,
  MethodsFn,
} from "./types.js";
import { ViewFn } from "./views.js";

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
    viewFn: ViewFn<IDL> | undefined,
    accountNamespace: AccountNamespace<IDL>,
    idlTypes: IdlTypeDef[],
    customResolver?: CustomAccountResolver<IDL>
  ): MethodsFn<IDL, I, MethodsBuilder<IDL, I>> {
    return (...args) =>
      new MethodsBuilder(
        args,
        ixFn,
        txFn,
        rpcFn,
        simulateFn,
        viewFn,
        provider,
        programId,
        idlIx,
        accountNamespace,
        idlTypes,
        customResolver
      );
  }
}

export type PartialAccounts<A extends IdlAccountItem = IdlAccountItem> =
  Partial<{
    [N in A["name"]]: PartialAccount<A & { name: N }>;
  }>;

type PartialAccount<A extends IdlAccountItem> = A extends IdlAccounts
  ? PartialAccounts<A["accounts"][number]>
  : A extends { isOptional: true }
  ? Address | null
  : Address;

export function isPartialAccounts(
  partialAccount: PartialAccount<IdlAccountItem>
): partialAccount is PartialAccounts {
  return (
    typeof partialAccount === "object" &&
    partialAccount !== null &&
    !("_bn" in partialAccount) // Ensures not a pubkey
  );
}

export function flattenPartialAccounts<A extends IdlAccountItem>(
  partialAccounts: PartialAccounts<A>,
  throwOnNull: boolean
): AccountsGeneric {
  const toReturn: AccountsGeneric = {};
  for (const accountName in partialAccounts) {
    const account = partialAccounts[accountName];
    if (account === null) {
      if (throwOnNull)
        throw new Error(
          "Failed to resolve optionals due to IDL type mismatch with input accounts!"
        );
      continue;
    }
    toReturn[accountName] = isPartialAccounts(account)
      ? flattenPartialAccounts(account, true)
      : translateAddress(account);
  }
  return toReturn;
}

export class MethodsBuilder<IDL extends Idl, I extends AllInstructions<IDL>> {
  private readonly _accounts: AccountsGeneric = {};
  private _remainingAccounts: Array<AccountMeta> = [];
  private _signers: Array<Signer> = [];
  private _preInstructions: Array<TransactionInstruction> = [];
  private _postInstructions: Array<TransactionInstruction> = [];
  private _accountsResolver: AccountsResolver<IDL>;
  private _autoResolveAccounts: boolean = true;
  private _args: Array<any>;

  constructor(
    _args: Array<any>,
    private _ixFn: InstructionFn<IDL>,
    private _txFn: TransactionFn<IDL>,
    private _rpcFn: RpcFn<IDL>,
    private _simulateFn: SimulateFn<IDL>,
    private _viewFn: ViewFn<IDL> | undefined,
    _provider: Provider,
    private _programId: PublicKey,
    _idlIx: AllInstructions<IDL>,
    _accountNamespace: AccountNamespace<IDL>,
    _idlTypes: IdlTypeDef[],
    _customResolver?: CustomAccountResolver<IDL>
  ) {
    this._args = _args;
    this._accountsResolver = new AccountsResolver(
      _args,
      this._accounts,
      _provider,
      _programId,
      _idlIx,
      _accountNamespace,
      _idlTypes,
      _customResolver
    );
  }

  public args(_args: Array<any>): void {
    this._args = _args;
    this._accountsResolver.args(_args);
  }

  public async pubkeys(): Promise<
    Partial<InstructionAccountAddresses<IDL, I>>
  > {
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }
    return this._accounts as unknown as Partial<
      InstructionAccountAddresses<IDL, I>
    >;
  }

  public accounts(
    accounts: PartialAccounts<I["accounts"][number]>
  ): MethodsBuilder<IDL, I> {
    this._autoResolveAccounts = true;
    this._accountsResolver.resolveOptionals(accounts);
    return this;
  }

  public accountsStrict(
    accounts: Accounts<I["accounts"][number]>
  ): MethodsBuilder<IDL, I> {
    this._autoResolveAccounts = false;
    this._accountsResolver.resolveOptionals(accounts);
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

  public async rpc(options?: ConfirmOptions): Promise<TransactionSignature> {
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }

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

  public async rpcAndKeys(options?: ConfirmOptions): Promise<{
    pubkeys: Partial<InstructionAccountAddresses<IDL, I>>;
    signature: TransactionSignature;
  }> {
    const pubkeys = await this.pubkeys();
    return {
      pubkeys,
      signature: await this.rpc(options),
    };
  }

  public async view(options?: ConfirmOptions): Promise<any> {
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }

    if (!this._viewFn) {
      throw new Error("Method does not support views");
    }

    // @ts-ignore
    return this._viewFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
      options: options,
    });
  }

  public async simulate(
    options?: ConfirmOptions
  ): Promise<SimulateResponse<any, any>> {
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }

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
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }

    // @ts-ignore
    return this._ixFn(...this._args, {
      accounts: this._accounts,
      signers: this._signers,
      remainingAccounts: this._remainingAccounts,
      preInstructions: this._preInstructions,
      postInstructions: this._postInstructions,
    });
  }

  /**
   * Convenient shortcut to get instructions and pubkeys via
   * const { pubkeys, instructions } = await prepare();
   */
  public async prepare(): Promise<{
    pubkeys: Partial<InstructionAccountAddresses<IDL, I>>;
    instruction: TransactionInstruction;
    signers: Signer[];
  }> {
    return {
      instruction: await this.instruction(),
      pubkeys: await this.pubkeys(),
      signers: await this._signers,
    };
  }

  public async transaction(): Promise<Transaction> {
    if (this._autoResolveAccounts) {
      await this._accountsResolver.resolve();
    }

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
