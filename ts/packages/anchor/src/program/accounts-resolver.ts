import camelCase from "camelcase";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { Idl, IdlSeed, IdlAccount } from "../idl.js";
import * as utf8 from "../utils/bytes/utf8.js";
import { TOKEN_PROGRAM_ID, ASSOCIATED_PROGRAM_ID } from "../utils/token.js";
import { AllInstructions } from "./namespace/types.js";
import Provider from "../provider.js";
import { AccountNamespace } from "./namespace/account.js";
import { coder } from "../spl/token";

// Populates a given accounts context with PDAs and common missing accounts.
export class AccountsResolver<IDL extends Idl, I extends AllInstructions<IDL>> {
  static readonly CONST_ACCOUNTS = {
    associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
    rent: SYSVAR_RENT_PUBKEY,
    systemProgram: SystemProgram.programId,
    tokenProgram: TOKEN_PROGRAM_ID,
  };

  private _accountStore: AccountStore<IDL>;

  constructor(
    private _args: Array<any>,
    private _accounts: { [name: string]: PublicKey },
    private _provider: Provider,
    private _programId: PublicKey,
    private _idlIx: AllInstructions<IDL>,
    _accountNamespace: AccountNamespace<IDL>
  ) {
    this._accountStore = new AccountStore(_provider, _accountNamespace);
  }

  // Note: We serially resolve PDAs one by one rather than doing them
  //       in parallel because there can be dependencies between
  //       addresses. That is, one PDA can be used as a seed in another.
  //
  // TODO: PDAs need to be resolved in topological order. For now, we
  //       require the developer to simply list the accounts in the
  //       correct order. But in future work, we should create the
  //       dependency graph and resolve automatically.
  //
  public async resolve() {
    for (let k = 0; k < this._idlIx.accounts.length; k += 1) {
      // Cast is ok because only a non-nested IdlAccount can have a seeds
      // cosntraint.
      const accountDesc = this._idlIx.accounts[k] as IdlAccount;
      const accountDescName = camelCase(accountDesc.name);

      // Signers default to the provider.
      if (accountDesc.isSigner && !this._accounts[accountDescName]) {
        // @ts-expect-error
        if (this._provider.wallet === undefined) {
          throw new Error(
            "This function requires the Provider interface implementor to have a 'wallet' field."
          );
        }
        // @ts-expect-error
        this._accounts[accountDescName] = this._provider.wallet.publicKey;
        continue;
      }

      // Common accounts are auto populated with magic names by convention.
      if (
        Reflect.has(AccountsResolver.CONST_ACCOUNTS, accountDescName) &&
        !this._accounts[accountDescName]
      ) {
        this._accounts[accountDescName] =
          AccountsResolver.CONST_ACCOUNTS[accountDescName];
      }
    }

    for (let k = 0; k < this._idlIx.accounts.length; k += 1) {
      // Cast is ok because only a non-nested IdlAccount can have a seeds
      // cosntraint.
      const accountDesc = this._idlIx.accounts[k] as IdlAccount;
      const accountDescName = camelCase(accountDesc.name);

      // PDA derived from IDL seeds.
      if (
        accountDesc.pda &&
        accountDesc.pda.seeds.length > 0 &&
        !this._accounts[accountDescName]
      ) {
        await this.autoPopulatePda(accountDesc);
        continue;
      }
    }
  }

  private async autoPopulatePda(accountDesc: IdlAccount) {
    if (!accountDesc.pda || !accountDesc.pda.seeds)
      throw new Error("Must have seeds");

    const seeds: Buffer[] = await Promise.all(
      accountDesc.pda.seeds.map((seedDesc: IdlSeed) => this.toBuffer(seedDesc))
    );

    const programId = await this.parseProgramId(accountDesc);
    const [pubkey] = await PublicKey.findProgramAddress(seeds, programId);

    this._accounts[camelCase(accountDesc.name)] = pubkey;
  }

  private async parseProgramId(accountDesc: IdlAccount): Promise<PublicKey> {
    if (!accountDesc.pda?.programId) {
      return this._programId;
    }
    switch (accountDesc.pda.programId.kind) {
      case "const":
        return new PublicKey(
          this.toBufferConst(accountDesc.pda.programId.value)
        );
      case "arg":
        return this.argValue(accountDesc.pda.programId);
      case "account":
        return await this.accountValue(accountDesc.pda.programId);
      default:
        throw new Error(
          `Unexpected program seed kind: ${accountDesc.pda.programId.kind}`
        );
    }
  }

  private async toBuffer(seedDesc: IdlSeed): Promise<Buffer> {
    switch (seedDesc.kind) {
      case "const":
        return this.toBufferConst(seedDesc);
      case "arg":
        return await this.toBufferArg(seedDesc);
      case "account":
        return await this.toBufferAccount(seedDesc);
      default:
        throw new Error(`Unexpected seed kind: ${seedDesc.kind}`);
    }
  }

  private toBufferConst(seedDesc: IdlSeed): Buffer {
    return this.toBufferValue(seedDesc.type, seedDesc.value);
  }

  private async toBufferArg(seedDesc: IdlSeed): Promise<Buffer> {
    const argValue = this.argValue(seedDesc);
    return this.toBufferValue(seedDesc.type, argValue);
  }

  private argValue(seedDesc: IdlSeed): any {
    const seedArgName = camelCase(seedDesc.path.split(".")[0]);

    const idlArgPosition = this._idlIx.args.findIndex(
      (argDesc: any) => argDesc.name === seedArgName
    );
    if (idlArgPosition === -1) {
      throw new Error(`Unable to find argument for seed: ${seedArgName}`);
    }

    return this._args[idlArgPosition];
  }

  private async toBufferAccount(seedDesc: IdlSeed): Promise<Buffer> {
    const accountValue = await this.accountValue(seedDesc);
    return this.toBufferValue(seedDesc.type, accountValue);
  }

  private async accountValue(seedDesc: IdlSeed): Promise<any> {
    const pathComponents = seedDesc.path.split(".");

    const fieldName = pathComponents[0];
    const fieldPubkey = this._accounts[camelCase(fieldName)];

    // The seed is a pubkey of the account.
    if (pathComponents.length === 1) {
      return fieldPubkey;
    }

    // The key is account data.
    //
    // Fetch and deserialize it.
    const account = await this._accountStore.fetchAccount(
      seedDesc.account,
      fieldPubkey
    );

    // Dereference all fields in the path to get the field value
    // used in the seed.
    const fieldValue = this.parseAccountValue(account, pathComponents.slice(1));
    return fieldValue;
  }

  private parseAccountValue<T = any>(account: T, path: Array<string>): any {
    let accountField: any;
    while (path.length > 0) {
      accountField = account[camelCase(path[0])];
      path = path.slice(1);
    }
    return accountField;
  }

  // Converts the given idl valaue into a Buffer. The values here must be
  // primitives. E.g. no structs.
  //
  // TODO: add more types here as needed.
  private toBufferValue(type: string | any, value: any): Buffer {
    switch (type) {
      case "u8":
        return Buffer.from([value]);
      case "u16":
        let b = Buffer.alloc(2);
        b.writeUInt16LE(value);
        return b;
      case "u32":
        let buf = Buffer.alloc(4);
        buf.writeUInt32LE(value);
        return buf;
      case "u64":
        let bU64 = Buffer.alloc(8);
        bU64.writeBigUInt64LE(BigInt(value));
        return bU64;
      case "string":
        return Buffer.from(utf8.encode(value));
      case "publicKey":
        return value.toBuffer();
      default:
        if (type.array) {
          return Buffer.from(value);
        }
        throw new Error(`Unexpected seed type: ${type}`);
    }
  }
}

// TODO: this should be configureable to avoid unnecessary requests.
export class AccountStore<IDL extends Idl> {
  private _cache = new Map<string, any>();

  // todo: don't use the progrma use the account namespace.
  constructor(
    private _provider: Provider,
    private _accounts: AccountNamespace<IDL>
  ) {}

  public async fetchAccount<T = any>(
    name: string,
    publicKey: PublicKey
  ): Promise<T> {
    const address = publicKey.toString();
    if (!this._cache.has(address)) {
      if (name === "TokenAccount") {
        const accountInfo = await this._provider.connection.getAccountInfo(
          publicKey
        );
        if (accountInfo === null) {
          throw new Error(`invalid account info for ${address}`);
        }
        const data = coder().accounts.decode("token", accountInfo.data);
        this._cache.set(address, data);
      } else {
        const account = this._accounts[camelCase(name)].fetch(publicKey);
        this._cache.set(address, account);
      }
    }
    return this._cache.get(address);
  }
}
