import bs58 from "bs58";
import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import { Idl, IdlDiscriminator } from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { AccountsCoder } from "../index.js";

function bytesEqual(a: Uint8Array, b: Uint8Array): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i += 1) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}

/**
 * Encodes and decodes account objects.
 */
export class BorshAccountsCoder<A extends string = string>
  implements AccountsCoder
{
  /**
   * Maps account type identifier to a layout.
   */
  private accountLayouts: Map<
    A,
    { discriminator: IdlDiscriminator; layout: Layout }
  >;

  public constructor(private idl: Idl) {
    if (!idl.accounts) {
      this.accountLayouts = new Map();
      return;
    }

    const types = idl.types;
    if (!types) {
      throw new Error("Accounts require `idl.types`");
    }

    const layouts = idl.accounts.map((acc) => {
      const typeDef = types.find((ty) => ty.name === acc.name);
      if (!typeDef) {
        throw new Error(`Account not found: ${acc.name}`);
      }
      return [
        acc.name as A,
        {
          discriminator: acc.discriminator,
          layout: IdlCoder.typeDefLayout({ typeDef, types }),
        },
      ] as const;
    });

    this.accountLayouts = new Map(layouts);
  }

  public async encode<T = any>(accountName: A, account: T): Promise<Buffer> {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.accountLayouts.get(accountName);
    if (!layout) {
      throw new Error(`Unknown account: ${accountName}`);
    }
    const len = layout.layout.encode(account, buffer);
    const accountData = buffer.slice(0, len);
    const discriminator = this.accountDiscriminator(accountName);
    return Buffer.from([...discriminator, ...accountData]);
  }

  public decode<T = any>(accountName: A, data: Buffer): T {
    // Assert the account discriminator is correct.
    const discriminator = this.accountDiscriminator(accountName);
    const givenDisc = Uint8Array.from(data.subarray(0, discriminator.length));
    if (!bytesEqual(Uint8Array.from(discriminator), givenDisc)) {
      throw new Error("Invalid account discriminator");
    }
    return this.decodeUnchecked(accountName, data);
  }

  public decodeAny<T = any>(data: Buffer): T {
    for (const [name, layout] of this.accountLayouts) {
      const givenDisc = Uint8Array.from(
        data.subarray(0, layout.discriminator.length)
      );
      const matches = bytesEqual(
        Uint8Array.from(layout.discriminator),
        givenDisc
      );
      if (matches) return this.decodeUnchecked(name, data);
    }

    throw new Error("Account not found");
  }

  public decodeUnchecked<T = any>(accountName: A, acc: Buffer): T {
    // Chop off the discriminator before decoding.
    const discriminator = this.accountDiscriminator(accountName);
    const data = acc.subarray(discriminator.length);
    const layout = this.accountLayouts.get(accountName);
    if (!layout) {
      throw new Error(`Unknown account: ${accountName}`);
    }
    return layout.layout.decode(data);
  }

  public memcmp(accountName: A, appendData?: Buffer): any {
    const discriminator = this.accountDiscriminator(accountName);
    const bytes = appendData
      ? Uint8Array.from([...discriminator, ...appendData])
      : Uint8Array.from(discriminator);
    return {
      offset: 0,
      bytes: bs58.encode(Buffer.from(bytes)),
    };
  }

  public size(accountName: A): number {
    return (
      this.accountDiscriminator(accountName).length +
      IdlCoder.typeSize({ defined: { name: accountName } }, this.idl)
    );
  }

  /**
   * Get the unique discriminator prepended to all anchor accounts.
   *
   * @param name The name of the account to get the discriminator of.
   */
  public accountDiscriminator(name: string): Buffer {
    const account = this.idl.accounts?.find((acc) => acc.name === name);
    if (!account) {
      throw new Error(`Account not found: ${name}`);
    }

    return Buffer.from(account.discriminator);
  }
}
