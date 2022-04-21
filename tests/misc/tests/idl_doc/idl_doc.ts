import * as anchor from "@project-serum/anchor";
import { Program, Wallet } from "@project-serum/anchor";
import { IdlDoc } from "../../target/types/idl_doc";
const { expect } = require("chai");
const idl_doc_idl = require("../../target/idl/idl_doc.json");

describe("idl_doc", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  const wallet = provider.wallet as Wallet;
  anchor.setProvider(provider);
  const program = anchor.workspace.IdlDoc as Program<IdlDoc>;

  describe("IDL doc strings", () => {
    const instruction = program.idl.instructions.find(
      (i) => i.name === "testIdlDocParse"
    );
    it("includes instruction doc comment", async () => {
      expect(instruction.docs).to.have.same.members([
        "This instruction doc should appear in the IDL",
      ]);
    });

    it("includes account doc comment", async () => {
      const act = instruction.accounts.find((i) => i.name === "act");
      expect(act.docs).to.have.same.members([
        "This account doc comment should appear in the IDL",
        "This is a multi-line comment",
      ]);
    });

    const dataWithDoc = program.idl.accounts.find(
      // @ts-expect-error
      (i) => i.name === "DataWithDoc"
    );

    it("includes accounts doc comment", async () => {
      expect(dataWithDoc.docs).to.have.same.members([
        "Custom account doc comment should appear in the IDL",
      ]);
    });

    it("includes account attribute doc comment", async () => {
      const dataField = dataWithDoc.type.fields.find((i) => i.name === "data");
      expect(dataField.docs).to.have.same.members([
        "Account attribute doc comment should appear in the IDL",
      ]);
    });
  });
});
