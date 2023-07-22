import * as anchor from "@coral-xyz/anchor";

import { IDL } from "../target/types/idl";

describe(IDL.name, () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  it("Builds", () => {});
});
