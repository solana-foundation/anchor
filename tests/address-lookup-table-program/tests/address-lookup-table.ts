import * as anchor from "@coral-xyz/anchor";
import { AnchorError, Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { assert } from "chai";
import {
  AddressLookupTable,
  AddressLookupTableProgram,
} from "../target/types/address_lookup_table_program";

describe("address_lookup_table_program", () => {
  const provider = anchor.AnchorProvider.env();
  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const program = anchor.workspace
    .AddressLookupTableProgram as Program<AddressLookupTableProgram>;
  const lutProgramAddress = new anchor.web3.PublicKey(
    "AddressLookupTab1e1111111111111111111111111"
  );

  it("Test loads", async () => {
    const tx = await program.rpc.test({
      accounts: {
        authority: provider.wallet.publicKey,
        lutProgram: lutProgramAddress,
        table: lutProgramAddress, // Just a dummy value. fix
      },
      signers: [settings],
    });
  });
});
