import { type Address, getAddressDecoder } from '@solana/kit';

import { InstructionCodec } from '../codec/instruction-codec';
import { IdlError } from '../error';
import type {
  Idl,
  InstructionAccountName,
  InstructionArgs,
  InstructionByName,
  InstructionName,
} from '../types';
import { flattenInstructionAccounts } from '../utils/instruction-accounts';

export class InstructionClient<IDL extends Idl, N extends InstructionName<IDL>> {
  private readonly codec: InstructionCodec<InstructionArgs<IDL, N>>;
  private readonly accountIndexByName: Map<string, number>;

  constructor(
    private readonly idl: IDL,
    private readonly instructionDef: InstructionByName<IDL, N>,
  ) {
    if (instructionDef.name === '_inner') {
      throw new IdlError('The _inner name is reserved');
    }

    this.accountIndexByName = new Map(
      flattenInstructionAccounts(instructionDef.accounts).map((account, index) => [
        account.name,
        index,
      ]),
    );

    this.codec = new InstructionCodec<InstructionArgs<IDL, N>>(this.idl, this.instructionDef);
  }

  get discriminator(): Uint8Array {
    return Uint8Array.from(this.instructionDef.discriminator);
  }

  encode(data: InstructionArgs<IDL, N>): Uint8Array {
    return this.codec.encode(data);
  }

  decode(data: Uint8Array): InstructionArgs<IDL, N> {
    return this.codec.decode(data);
  }

  getAccount(
    accountName: InstructionAccountName<IDL, N>,
    instructionAccounts: Uint8Array[],
  ): Address<string> {
    const acc = instructionAccounts[this.accountIndexByName.get(accountName)!];

    if (!acc) {
      throw new IdlError('Account not found');
    }

    return getAddressDecoder().decode(acc);
  }
}
