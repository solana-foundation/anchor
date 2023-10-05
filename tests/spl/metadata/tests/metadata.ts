import * as anchor from "@coral-xyz/anchor";

import { Metadata } from "../target/types/metadata";

describe("Client interactions", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.metadata as anchor.Program<Metadata>;

  it("Builds and deploys", () => {
    console.log("Program ID:", program.programId.toBase58());
  });
});
