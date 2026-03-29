import { decodeIdlConstValue } from '../codec/type-codec';
import type { ConstantByName, ConstantName, DecodedConstant, Idl } from '../types';

export class ConstantClient<IDL extends Idl, N extends ConstantName<IDL>> {
  constructor(
    private readonly idl: IDL,
    private readonly constantDef: ConstantByName<IDL, N>,
  ) {}

  get value() {
    return decodeIdlConstValue(
      this.constantDef.type,
      this.constantDef.value,
      this.idl.types ?? [],
    ) as DecodedConstant<IDL, N>;
  }
}
