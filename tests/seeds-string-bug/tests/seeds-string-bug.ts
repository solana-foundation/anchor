import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SeedsStringBug } from "../target/types/seeds_string_bug";
import { assert, expect } from "chai";
import { PublicKey } from "@solana/web3.js";

describe("seeds-string-bug", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.SeedsStringBug as Program<SeedsStringBug>;

  it("Can create cohort with string seeds after fix", async () => {
    const cohortName = "EXACTTEST";
    const year = 2025;
    const sport = "football";
    const clubName = "test-club";
    const maxSupply = 1000;
    const mintPrice = 100;

    // Manual PDA derivation (как это делает клиент)
    const yearBytes = Buffer.allocUnsafe(2);
    yearBytes.writeUInt16LE(year, 0);
    const [expectedPda, bump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("cohort"),      // b"cohort"
        Buffer.from(cohortName),    // cohort_name.as_bytes()
        yearBytes                   // year.to_le_bytes().as_ref()
      ],
      program.programId
    );

    console.log("Manual PDA derivation:", expectedPda.toString());
    console.log("Bump:", bump);

    // Теперь это должно работать без ошибок
    const tx = await program.methods
      .initializeCohort(cohortName, sport, year, clubName, maxSupply, mintPrice)
      .accounts({
        cohort: expectedPda,
        authority: anchor.AnchorProvider.env().wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    console.log("Transaction successful:", tx);

    // Проверяем, что аккаунт был создан правильно
    const cohortAccount = await program.account.cohort.fetch(expectedPda);
    assert.equal(cohortAccount.cohortName, cohortName);
    assert.equal(cohortAccount.year, year);
    assert.equal(cohortAccount.sport, sport);
    assert.equal(cohortAccount.clubName, clubName);
    assert.equal(cohortAccount.maxSupply.toNumber(), maxSupply);
    assert.equal(cohortAccount.mintPrice.toNumber(), mintPrice);
    assert.equal(cohortAccount.bump, bump);
  });

  it("Can validate existing cohort PDA", async () => {
    const cohortName = "EXACTTEST";
    const year = 2025;

    // Manual PDA derivation
    const yearBytes = Buffer.allocUnsafe(2);
    yearBytes.writeUInt16LE(year, 0);
    const [expectedPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("cohort"),
        Buffer.from(cohortName),
        yearBytes
      ],
      program.programId
    );

    // Теперь это должно работать без ошибок seeds constraint
    await program.methods
      .testPdaDerivation(cohortName, year)
      .accounts({
        cohort: expectedPda,
      })
      .rpc();

    console.log("PDA validation successful");
  });

  it("Tests different cohort names and years", async () => {
    const testCases = [
      { cohortName: "TEST", year: 2024 },
      { cohortName: "BUCC", year: 2026 },
      { cohortName: "SHORT", year: 2023 },
      { cohortName: "LONGNAMETEST", year: 2027 },
    ];

    for (const testCase of testCases) {
      const { cohortName, year } = testCase;
      
      const yearBytes = Buffer.allocUnsafe(2);
      yearBytes.writeUInt16LE(year, 0);
      const [expectedPda] = PublicKey.findProgramAddressSync(
        [
          Buffer.from("cohort"),
          Buffer.from(cohortName),
          yearBytes
        ],
        program.programId
      );

      // Проверяем, что PDA derivation работает
      await program.methods
        .testPdaDerivation(cohortName, year)
        .accounts({
          cohort: expectedPda,
        })
        .simulate(); // Используем simulate для проверки без создания транзакции

      console.log(`PDA validation successful for ${cohortName}-${year}`);
    }
  });
}); 