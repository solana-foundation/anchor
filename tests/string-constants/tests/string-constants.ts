import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { StringConstants } from "../target/types/string_constants";
import { expect } from "chai";
describe("string-constants", () => {
    const provider = anchor.AnchorProvider.env();
    anchor.setProvider(provider);

    const program = anchor.workspace.StringConstants as Program<StringConstants>;

    it("Verifies correct generation of string constants in IDL", () => {
        const idl = program.rawIdl;
        
        // Find constants in IDL
        const stringConst = idl.constants?.find(c => c.name === "STRING_CONST");
        const stringWithQuotes = idl.constants?.find(c => c.name === "STRING_WITH_QUOTES");
        const stringWithEscapes = idl.constants?.find(c => c.name === "STRING_WITH_ESCAPES");
        const numberConst = idl.constants?.find(c => c.name === "NUMBER_CONST");
        const bytesConst = idl.constants?.find(c => c.name === "BYTES_CONST");

        // Test string constants
        expect(stringConst).to.not.be.undefined;
        expect(stringConst?.value).to.equal('"test string"');
        
        expect(stringWithQuotes).to.not.be.undefined;
        expect(stringWithQuotes?.value).to.equal('"test \\"quoted\\" string"');
        
        expect(stringWithEscapes).to.not.be.undefined;
        expect(stringWithEscapes?.value).to.equal('"test\\nstring\\twith\\rescapes"');

        // Test backward compatibility with other types
        expect(numberConst).to.not.be.undefined;
        expect(numberConst?.value).to.equal("12345");
        
        expect(bytesConst).to.not.be.undefined;
        expect(bytesConst?.value).to.equal("[116, 101, 115, 116, 32, 98, 121, 116, 101, 115]");
    });
}); 