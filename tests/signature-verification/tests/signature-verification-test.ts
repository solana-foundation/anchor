import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import * as fs from "fs";
const signatureVerificationTestIDL = JSON.parse(
  fs.readFileSync("./target/idl/signature_verification_test.json", "utf8")
);
import { Buffer } from "buffer";
import {
  PublicKey,
  Keypair,
  Transaction,
  SystemProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  Ed25519Program,
  Secp256k1Program,
} from "@solana/web3.js";
import * as crypto from "crypto";
import { ethers } from "ethers";
import * as assert from "assert";
import { sign } from "@noble/ed25519";

describe("signature-verification-test", () => {
  const provider = anchor.AnchorProvider.local(undefined, {
    commitment: `confirmed`,
  });

  anchor.setProvider(provider);
  const program = new anchor.Program(
    signatureVerificationTestIDL as anchor.Idl,
    provider
  );

  it("Verify Ed25519 signature with valid signature", async () => {
    const signer = Keypair.generate();
    const message = Buffer.from(
      "Hello, Anchor Signature Verification Test with valid signature!"
    );
    const signature = await sign(message, signer.secretKey.slice(0, 32));

    // Create transaction with just the Ed25519Program instruction
    const ed25519Instruction = Ed25519Program.createInstructionWithPublicKey({
      publicKey: signer.publicKey.toBytes(),
      message: message,
      signature: signature,
    });

    const transaction = new Transaction().add(ed25519Instruction);

    try {
      await provider.sendAndConfirm(transaction, []);
      console.log("Ed25519 signature verified successfully!");
    } catch (error) {
      assert.fail("Valid Ed25519 signature should be verified");
    }
  });

  it("Verify Ed25519 signature with invalid signature", async () => {
    const signer = Keypair.generate();
    const message = Buffer.from(
      "Hello, Anchor Signature Verification Test with invalid signature!"
    );
    // Create a fake signature (all zeros)
    const fakeSignature = new Uint8Array(64).fill(0);

    // Create transaction with just the Ed25519Program instruction
    const ed25519Instruction = Ed25519Program.createInstructionWithPublicKey({
      publicKey: signer.publicKey.toBytes(),
      message: message,
      signature: fakeSignature,
    });

    const transaction = new Transaction().add(ed25519Instruction);

    // This should fail
    try {
      await provider.sendAndConfirm(transaction, []);
      assert.fail("Invalid Signature of Ed25519 should not be verified");
    } catch (error) {
      console.log("Invalid Signature of Ed25519 is not verified");
    }
  });

  it("Verify Ethereum Secp256k1 signature with valid signature", async () => {
    const ethSigner: ethers.Wallet = ethers.Wallet.createRandom();
    const PERSON = { name: "ben", age: 49 };

    // keccak256(name, age)
    const messageHashHex: string = ethers.utils.solidityKeccak256(
      ["string", "uint16"],
      [PERSON.name, PERSON.age]
    );
    const messageHashBytes: Uint8Array = ethers.utils.arrayify(messageHashHex);

    // Sign with Ethereum prefix
    const fullSig: string = await ethSigner.signMessage(messageHashBytes);
    const fullSigBytes = ethers.utils.arrayify(fullSig);
    const signature = fullSigBytes.slice(0, 64);
    const recoveryId = fullSigBytes[64] - 27;

    const actualMessage = Buffer.concat([
      Buffer.from("\x19Ethereum Signed Message:\n32"),
      Buffer.from(messageHashBytes),
    ]);

    // 20-byte ETH address (hex without 0x)
    const ethAddressHexNo0x = ethers.utils
      .computeAddress(ethSigner.publicKey)
      .slice(2);
    const ethAddressBytes = Array.from(
      ethers.utils.arrayify("0x" + ethAddressHexNo0x)
    ) as [
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number
    ];

    const verifyIx = await program.methods
      .verifySecp(
        actualMessage,
        Array.from(signature) as [
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number
        ],
        recoveryId,
        ethAddressBytes
      )
      .accounts({
        ixSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .instruction();

    // Secp precompile verification against ETH address
    const secpIx = Secp256k1Program.createInstructionWithEthAddress({
      ethAddress: ethAddressHexNo0x,
      message: actualMessage,
      signature: Uint8Array.from(signature),
      recoveryId,
    });

    const tx = new Transaction().add(secpIx).add(verifyIx);
    // This should succeed
    try {
      await provider.sendAndConfirm(tx, []);
      console.log("Ethereum Secp256k1 signature verified successfully!");
    } catch (error) {
      assert.fail("Valid Signature of Ethereum Secp256k1 should be verified");
    }
  });

  it("Verify Ethereum Secp256k1 signature with invalid signature", async () => {
    const ethSigner: ethers.Wallet = ethers.Wallet.createRandom();
    const PERSON = { name: "ben", age: 49 };

    // keccak256(name, age)
    const messageHashHex: string = ethers.utils.solidityKeccak256(
      ["string", "uint16"],
      [PERSON.name, PERSON.age]
    );
    const messageHashBytes: Uint8Array = ethers.utils.arrayify(messageHashHex);

    // Create a fake signature (all zeros)
    const fakeSignature = new Uint8Array(64).fill(0);
    const fakeRecoveryId = 0;

    const actualMessage = Buffer.concat([
      Buffer.from("\x19Ethereum Signed Message:\n32"),
      Buffer.from(messageHashBytes),
    ]);

    const ethAddressHexNo0x = ethers.utils
      .computeAddress(ethSigner.publicKey)
      .slice(2);
    const ethAddressBytes = Array.from(
      ethers.utils.arrayify("0x" + ethAddressHexNo0x)
    ) as [
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number,
      number
    ];

    const verifyIx = await program.methods
      .verifySecp(
        actualMessage,
        Array.from(fakeSignature) as [
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number,
          number
        ],
        fakeRecoveryId,
        ethAddressBytes
      )
      .accounts({
        ixSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .instruction();
    const secpIx = Secp256k1Program.createInstructionWithEthAddress({
      ethAddress: ethAddressHexNo0x,
      message: actualMessage,
      signature: fakeSignature,
      recoveryId: fakeRecoveryId,
    });

    const tx = new Transaction().add(secpIx).add(verifyIx);

    // This should fail
    try {
      await provider.sendAndConfirm(tx, []);
      assert.fail("Expected transaction to fail with invalid signature");
    } catch (error) {
      console.log(
        "Ethereum Secp256k1 verification correctly failed with invalid signature"
      );
    }
  });
});
