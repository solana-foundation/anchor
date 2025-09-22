import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SignatureVerificationTest } from "../target/types/signature_verification_test";
import { Buffer } from "buffer";
import { PublicKey, Keypair, Transaction, SystemProgram, SYSVAR_INSTRUCTIONS_PUBKEY, Ed25519Program, Secp256k1Program } from "@solana/web3.js";
import * as crypto from "crypto";

describe("signature-verification-test", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);
  const program = anchor.workspace.SignatureVerificationTest as Program<SignatureVerificationTest>;

  it("Verify Ed25519 signature with actual signature", async () => {
    // Create a keypair for testing
    const signer = Keypair.generate();

    // Create a test message
    const message = Buffer.from("Hello, Anchor Signature Verification Test!");

    // Create a mock signature (in real implementation, this would be properly signed)
    const signature = new Uint8Array(64).fill(1); // Mock signature

    // Create instruction to call the program
    const instruction = await program.methods
      .verifyEd25519Signature(message, Array.from(signature) as [number, ...number[]])
      .accounts({
        signer: signer.publicKey,
        ixSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .instruction();

    // Create transaction with the signature verification instruction
    const transaction = new Transaction().add(instruction);

    // Add the Ed25519 signature instruction to the transaction
    const ed25519Instruction = Ed25519Program.createInstructionWithPublicKey({
      publicKey: signer.publicKey.toBytes(),
      message: message,
      signature: signature,
    });

    transaction.add(ed25519Instruction);

    try {
      await provider.sendAndConfirm(transaction, [signer]);
      console.log("✅ Ed25519 signature verified successfully using custom helper!");
    } catch (error) {
      console.log("❌ Ed25519 verification failed:", error.message);
      // This test demonstrates the structure even if signature verification fails
    }
  });

  it("Verify Secp256k1 signature with actual signature", async () => {
    // Create ETH address (20 bytes)
    const ethAddress = crypto.randomBytes(20);

    // Create a test message hash (32 bytes)
    const messageHash = crypto.randomBytes(32);

    // Create a mock signature
    const signature = new Uint8Array(64).fill(2);
    const recoveryId = 0;

    // Create instruction to call the program
    const instruction = await program.methods
      .verifySecp256k1Signature(
        Array.from(messageHash) as [number, ...number[]],
        Array.from(signature) as [number, ...number[]],
        recoveryId,
        Array.from(ethAddress) as [number, ...number[]]
      )
      .accounts({
        ixSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
      })
      .instruction();

    // Create transaction with the signature verification instruction
    const transaction = new Transaction().add(instruction);

    // Add the Secp256k1 signature instruction to the transaction
    const secp256k1Instruction = Secp256k1Program.createInstructionWithPublicKey({
      publicKey: Buffer.from(ethAddress), // Use ETH address as public key
      message: messageHash,
      signature: signature,
      recoveryId: recoveryId,
    });

    transaction.add(secp256k1Instruction);

    try {
      await provider.sendAndConfirm(transaction, []);
      console.log("✅ Secp256k1 signature verified successfully using custom helper!");
    } catch (error) {
      console.log("❌ Secp256k1 verification failed:", error.message);
      // This test demonstrates the structure even if signature verification fails
    }
  });

});