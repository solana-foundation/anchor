import * as anchor from "@coral-xyz/anchor";

import { Lamports } from "../../target/types/lamports";

describe("lamports", () => {
  // Configure the client to use the local cluster
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.Lamports as anchor.Program<Lamports>;

  it("Can use the Lamports trait", async () => {
    const signer = program.provider.publicKey!;
    const [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("lamports")],
      program.programId
    );

    await program.methods
      .testLamportsTrait(new anchor.BN(anchor.web3.LAMPORTS_PER_SOL))
      .accounts({ signer, pda })
      .rpc();
  });
});
