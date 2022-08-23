import bs58 from "bs58";
import { Buffer } from "buffer";
import { Layout } from "buffer-layout";
import camelcase from "camelcase";
import { sha256 } from "js-sha256";
import { Idl, IdlTypeDef } from "../../idl.js";
import { IdlCoder } from "./idl.js";
import { AccountsCoder } from "../index.js";
import { accountSize } from "../common.js";

/**
 * Number of bytes of the account discriminator.
 */
export const ACCOUNT_DISCRIMINATOR_SIZE = 8;

/**
 * Encodes and decodes account objects.
 */
export class BorshAccountsCoder<A extends string = string>
  implements AccountsCoder
{
  /**
   * Maps account type identifier to a layout.
   */
  private accountLayouts: Map<A, Layout>;

  /**
   * IDL whose acconts will be coded.
   */
  private idl: Idl;

  public constructor(idl: Idl) {
    if (idl.accounts === undefined) {
      this.accountLayouts = new Map();
      return;
    }
    const layouts: [A, Layout][] = idl.accounts.map((acc) => {
      return [acc.name as A, IdlCoder.typeDefLayout(acc, idl.types)];
    });

    this.accountLayouts = new Map(layouts);
    this.idl = idl;
  }

  public async encode<T = any>(accountName: A, account: T): Promise<Buffer> {
    const buffer = Buffer.alloc(1000); // TODO: use a tighter buffer.
    const layout = this.accountLayouts.get(accountName);
    if (!layout) {
      throw new Error(`Unknown account: ${accountName}`);
    }
    const len = layout.encode(account, buffer);
    let accountData = buffer.slice(0, len);
    let discriminator = BorshAccountsCoder.accountDiscriminator(accountName);
    return Buffer.concat([discriminator, accountData]);
  }

  public decode<T = any>(accountName: A, data: Buffer): T {
    // Assert the account discriminator is correct.
    const discriminator = BorshAccountsCoder.accountDiscriminator(accountName);
    if (discriminator.compare(data.slice(0, 8))) {
      throw new Error("Invalid account discriminator");
    }
    return this.decodeUnchecked(accountName, data);
  }

  public decodeUnchecked<T = any>(accountName: A, ix: Buffer): T {
    // Chop off the discriminator before decoding.
    const data = ix.slice(ACCOUNT_DISCRIMINATOR_SIZE);
    const layout = this.accountLayouts.get(accountName);
    if (!layout) {
      throw new Error(`Unknown account: ${accountName}`);
    }
    return layout.decode(data);
  }

  public memcmp(accountName: A, appendData?: Buffer): any {
    const discriminator = BorshAccountsCoder.accountDiscriminator(accountName);
    return {
      offset: 0,
      bytes: bs58.encode(
        appendData ? Buffer.concat([discriminator, appendData]) : discriminator
      ),
    };
  }

  public size(idlAccount: IdlTypeDef): number {
    return (
      ACCOUNT_DISCRIMINATOR_SIZE + (accountSize(this.idl, idlAccount) ?? 0)
    );
  }

  /**
   * Calculates and returns a unique 8 byte discriminator prepended to all anchor accounts.
   *
   * @param name The name of the account to calculate the discriminator.
   */
  public static accountDiscriminator(name: string): Buffer {
    return Buffer.from(
      sha256.digest(`account:${camelcase(name, { pascalCase: true })}`)
    ).slice(0, ACCOUNT_DISCRIMINATOR_SIZE);
  }
}
