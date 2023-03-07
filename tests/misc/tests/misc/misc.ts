import * as anchor from "@coral-xyz/anchor";
import { Program, BN, AnchorError, Wallet, IdlEvents } from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  Message,
  VersionedTransaction,
  AddressLookupTableProgram,
  TransactionMessage,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  Token,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { Misc } from "../../target/types/misc";
import { MiscOptional } from "../../target/types/misc_optional";

const utf8 = anchor.utils.bytes.utf8;
const { assert, expect } = require("chai");
const nativeAssert = require("assert");
const miscIdl = require("../../target/idl/misc.json");

const miscTest = (
  program: anchor.Program<Misc> | anchor.Program<MiscOptional>
) => {
  return () => {
    // Configure the client to use the local cluster.
    const provider = anchor.AnchorProvider.env();
    const wallet = provider.wallet as Wallet;
    anchor.setProvider(provider);

    const data = anchor.web3.Keypair.generate();

    it("Can use u128 and i128", async () => {
      const tx = await program.rpc.initialize(
        new anchor.BN(1234),
        new anchor.BN(22),
        {
          accounts: {
            data: data.publicKey,
          },
          signers: [data],
          instructions: [await program.account.data.createInstruction(data)],
        }
      );
      const dataAccount = await program.account.data.fetch(data.publicKey);
      assert.isTrue(dataAccount.udata.eq(new anchor.BN(1234)));
      assert.isTrue(dataAccount.idata.eq(new anchor.BN(22)));
    });

    it("Can use u16", async () => {
      const data = anchor.web3.Keypair.generate();
      const tx = await program.rpc.testU16(99, {
        accounts: {
          myAccount: data.publicKey,
        },
        signers: [data],
        instructions: [await program.account.dataU16.createInstruction(data)],
      });
      const dataAccount = await program.account.dataU16.fetch(data.publicKey);
      assert.strictEqual(dataAccount.data, 99);
    });

    it("Can send VersionedTransaction", async () => {
      // Create the lookup table
      const recentSlot = await provider.connection.getSlot();
      const [loookupTableInstruction, lookupTableAddress] =
        AddressLookupTableProgram.createLookupTable({
          authority: provider.publicKey,
          payer: provider.publicKey,
          recentSlot,
        });
      const extendInstruction = AddressLookupTableProgram.extendLookupTable({
        payer: provider.publicKey,
        authority: provider.publicKey,
        lookupTable: lookupTableAddress,
        addresses: [provider.publicKey, SystemProgram.programId],
      });
      let createLookupTableTx = new VersionedTransaction(
        new TransactionMessage({
          instructions: [loookupTableInstruction, extendInstruction],
          payerKey: program.provider.publicKey,
          recentBlockhash: (await provider.connection.getLatestBlockhash())
            .blockhash,
        }).compileToV0Message()
      );
      type SendParams = Parameters<typeof provider.sendAndConfirm>;
      const testThis: SendParams = [
        new VersionedTransaction(
          new TransactionMessage({
            instructions: [loookupTableInstruction, extendInstruction],
            payerKey: program.provider.publicKey,
            recentBlockhash: (await provider.connection.getLatestBlockhash())
              .blockhash,
          }).compileToV0Message()
        ),
      ];
      await provider.sendAndConfirm(createLookupTableTx, [], {
        skipPreflight: true,
      });

      // Use the lookup table in a transaction
      const transferAmount = 1_000_000;
      const lookupTableAccount = await provider.connection
        .getAddressLookupTable(lookupTableAddress)
        .then((res) => res.value);
      const target = Keypair.generate();
      let transferInstruction = SystemProgram.transfer({
        fromPubkey: provider.publicKey,
        lamports: transferAmount,
        toPubkey: target.publicKey,
      });
      let transferUsingLookupTx = new VersionedTransaction(
        new TransactionMessage({
          instructions: [transferInstruction],
          payerKey: program.provider.publicKey,
          recentBlockhash: (await provider.connection.getLatestBlockhash())
            .blockhash,
        }).compileToV0Message([lookupTableAccount])
      );
      await provider.simulate(transferUsingLookupTx, [], "processed");
      await provider.sendAndConfirm(transferUsingLookupTx, [], {
        skipPreflight: true,
        commitment: "confirmed",
      });
      let newBalance = await provider.connection.getBalance(
        target.publicKey,
        "confirmed"
      );
      assert.strictEqual(newBalance, transferAmount);

      // Test sendAll with versioned transaction
      let oneTransferUsingLookupTx = new VersionedTransaction(
        new TransactionMessage({
          instructions: [
            SystemProgram.transfer({
              fromPubkey: provider.publicKey,
              // Needed to make the transactions distinct
              lamports: transferAmount + 1,
              toPubkey: target.publicKey,
            }),
          ],
          payerKey: program.provider.publicKey,
          recentBlockhash: (await provider.connection.getLatestBlockhash())
            .blockhash,
        }).compileToV0Message([lookupTableAccount])
      );
      let twoTransferUsingLookupTx = new VersionedTransaction(
        new TransactionMessage({
          instructions: [
            SystemProgram.transfer({
              fromPubkey: provider.publicKey,
              lamports: transferAmount,
              toPubkey: target.publicKey,
            }),
          ],
          payerKey: program.provider.publicKey,
          recentBlockhash: (await provider.connection.getLatestBlockhash())
            .blockhash,
        }).compileToV0Message([lookupTableAccount])
      );
      await provider.sendAll(
        [{ tx: oneTransferUsingLookupTx }, { tx: twoTransferUsingLookupTx }],
        { skipPreflight: true, commitment: "confirmed" }
      );
      newBalance = await provider.connection.getBalance(
        target.publicKey,
        "confirmed"
      );
      assert.strictEqual(newBalance, transferAmount * 3 + 1);
    });

    it("Can send VersionedTransaction with extra signatures", async () => {
      // Test sending with signatures
      const initSpace = 100;
      const rentExemptAmount =
        await provider.connection.getMinimumBalanceForRentExemption(initSpace);

      const newAccount = Keypair.generate();
      let createAccountIx = SystemProgram.createAccount({
        fromPubkey: provider.publicKey,
        lamports: rentExemptAmount,
        newAccountPubkey: newAccount.publicKey,
        programId: program.programId,
        space: initSpace,
      });
      let createAccountTx = new VersionedTransaction(
        new TransactionMessage({
          instructions: [createAccountIx],
          payerKey: provider.publicKey,
          recentBlockhash: (await provider.connection.getLatestBlockhash())
            .blockhash,
        }).compileToV0Message()
      );
      await provider.simulate(createAccountTx, [], "processed");
      await provider.sendAndConfirm(createAccountTx, [newAccount], {
        skipPreflight: false,
        commitment: "confirmed",
      });
      let newAccountInfo = await provider.connection.getAccountInfo(
        newAccount.publicKey
      );
      assert.strictEqual(
        newAccountInfo.owner.toBase58(),
        program.programId.toBase58()
      );
    });

    it("Can embed programs into genesis from the Anchor.toml", async () => {
      const pid = new anchor.web3.PublicKey(
        "FtMNMKp9DZHKWUyVAsj3Q5QV8ow4P3fUPP7ZrWEQJzKr"
      );
      let accInfo = await anchor.getProvider().connection.getAccountInfo(pid);
      assert.isTrue(accInfo.executable);
    });

    it("Can use the owner constraint", async () => {
      await program.rpc.testOwner({
        accounts: {
          data: data.publicKey,
          misc: program.programId,
        },
      });

      await nativeAssert.rejects(
        async () => {
          await program.rpc.testOwner({
            accounts: {
              data: provider.wallet.publicKey,
              misc: program.programId,
            },
          });
        },
        (err) => {
          return true;
        }
      );
    });

    it("Can use the executable attribute", async () => {
      await program.rpc.testExecutable({
        accounts: {
          program: program.programId,
        },
      });

      await nativeAssert.rejects(
        async () => {
          await program.rpc.testExecutable({
            accounts: {
              program: provider.wallet.publicKey,
            },
          });
        },
        (err) => {
          return true;
        }
      );
    });

    it("Can retrieve events when simulating a transaction", async () => {
      const resp = await program.methods.testSimulate(44).simulate();
      const expectedRaw = [
        `Program ${program.programId.toString()} invoke [1]`,
        "Program log: Instruction: TestSimulate",
        "Program data: NgyCA9omwbMsAAAA",
        "Program data: fPhuIELK/k7SBAAA",
        "Program data: jvbowsvlmkcJAAAA",
        "Program data: zxM5neEnS1kBAgMEBQYHCAkK",
        "Program data: g06Ei2GL1gIBAgMEBQYHCAkKCw==",
      ];

      assert.deepStrictEqual(expectedRaw, resp.raw.slice(0, -2));
      assert.strictEqual(resp.events[0].name, "E1");
      assert.strictEqual(resp.events[0].data.data, 44);
      assert.strictEqual(resp.events[1].name, "E2");
      assert.strictEqual(resp.events[1].data.data, 1234);
      assert.strictEqual(resp.events[2].name, "E3");
      assert.strictEqual(resp.events[2].data.data, 9);
      assert.strictEqual(resp.events[3].name, "E5");
      assert.deepStrictEqual(
        resp.events[3].data.data,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
      );
      assert.strictEqual(resp.events[4].name, "E6");
      assert.deepStrictEqual(
        resp.events[4].data.data,
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]
      );
    });

    it("Can use enum in idl", async () => {
      const resp1 = await program.methods
        .testInputEnum({ first: {} })
        .simulate();
      const event1 = resp1.events[0].data as IdlEvents<
        typeof program.idl
      >["E7"];
      assert.deepEqual(event1.data.first, {});

      const resp2 = await program.methods
        .testInputEnum({ second: { x: new BN(1), y: new BN(2) } })
        .simulate();
      const event2 = resp2.events[0].data as IdlEvents<
        typeof program.idl
      >["E7"];
      assert.isTrue(new BN(1).eq(event2.data.second.x));
      assert.isTrue(new BN(2).eq(event2.data.second.y));

      const resp3 = await program.methods
        .testInputEnum({
          tupleStructTest: [
            { data1: 1, data2: 11, data3: 111, data4: new BN(1111) },
          ],
        })
        .simulate();
      const event3 = resp3.events[0].data as IdlEvents<
        typeof program.idl
      >["E7"];
      assert.strictEqual(event3.data.tupleStructTest[0].data1, 1);
      assert.strictEqual(event3.data.tupleStructTest[0].data2, 11);
      assert.strictEqual(event3.data.tupleStructTest[0].data3, 111);
      assert.isTrue(event3.data.tupleStructTest[0].data4.eq(new BN(1111)));

      const resp4 = await program.methods
        .testInputEnum({ tupleTest: [1, 2, 3, 4] })
        .simulate();
      const event4 = resp4.events[0].data as IdlEvents<
        typeof program.idl
      >["E7"];
      assert.strictEqual(event4.data.tupleTest[0], 1);
      assert.strictEqual(event4.data.tupleTest[1], 2);
      assert.strictEqual(event4.data.tupleTest[2], 3);
      assert.strictEqual(event4.data.tupleTest[3], 4);
    });

    let dataI8;

    it("Can use i8 in the idl", async () => {
      dataI8 = anchor.web3.Keypair.generate();
      await program.rpc.testI8(-3, {
        accounts: {
          data: dataI8.publicKey,
        },
        instructions: [await program.account.dataI8.createInstruction(dataI8)],
        signers: [dataI8],
      });
      const dataAccount = await program.account.dataI8.fetch(dataI8.publicKey);
      assert.strictEqual(dataAccount.data, -3);
    });

    let dataPubkey;

    it("Can use i16 in the idl", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testI16(-2048, {
        accounts: {
          data: data.publicKey,
        },
        instructions: [await program.account.dataI16.createInstruction(data)],
        signers: [data],
      });
      const dataAccount = await program.account.dataI16.fetch(data.publicKey);
      assert.strictEqual(dataAccount.data, -2048);

      dataPubkey = data.publicKey;
    });

    it("Can use base58 strings to fetch an account", async () => {
      const dataAccount = await program.account.dataI16.fetch(
        dataPubkey.toString()
      );
      assert.strictEqual(dataAccount.data, -2048);
    });

    it("Should fail to close an account when sending lamports to itself", async () => {
      try {
        await program.rpc.testClose({
          accounts: {
            data: data.publicKey,
            solDest: data.publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (err) {
        const errMsg = "A close constraint was violated";
        assert.strictEqual(err.error.errorMessage, errMsg);
        assert.strictEqual(err.error.errorCode.number, 2011);
      }
    });

    it("Can close an account", async () => {
      const connection = program.provider.connection;
      const openAccount = await connection.getAccountInfo(data.publicKey);

      assert.isNotNull(openAccount);
      const openAccountBalance = openAccount.lamports;
      // double balance to calculate closed balance correctly
      const transferIx = anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: data.publicKey,
        lamports: openAccountBalance,
      });
      const transferTransaction = new anchor.web3.Transaction().add(transferIx);
      await provider.sendAndConfirm(transferTransaction);

      let beforeBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      await program.methods
        .testClose()
        .accounts({
          data: data.publicKey,
          solDest: provider.wallet.publicKey,
        })
        .postInstructions([transferIx])
        .rpc();

      let afterBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      // Retrieved rent exemption sol.
      expect(afterBalance > beforeBalance).to.be.true;

      const closedAccount = await connection.getAccountInfo(data.publicKey);

      assert.isTrue(closedAccount.data.length === 0);
      assert.isTrue(closedAccount.owner.equals(SystemProgram.programId));
    });

    it("Can close an account twice", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.methods
        .initialize(new anchor.BN(10), new anchor.BN(10))
        .accounts({ data: data.publicKey })
        .preInstructions([await program.account.data.createInstruction(data)])
        .signers([data])
        .rpc();

      const connection = program.provider.connection;
      const openAccount = await connection.getAccountInfo(data.publicKey);
      assert.isNotNull(openAccount);

      const openAccountBalance = openAccount.lamports;
      // double balance to calculate closed balance correctly
      const transferIx = anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: data.publicKey,
        lamports: openAccountBalance,
      });
      const transferTransaction = new anchor.web3.Transaction().add(transferIx);
      await provider.sendAndConfirm(transferTransaction);

      let beforeBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      await program.methods
        .testCloseTwice()
        .accounts({
          data: data.publicKey,
          solDest: provider.wallet.publicKey,
        })
        .postInstructions([transferIx])
        .rpc();

      let afterBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      // Retrieved rent exemption sol.
      expect(afterBalance > beforeBalance).to.be.true;

      const closedAccount = await connection.getAccountInfo(data.publicKey);
      assert.isTrue(closedAccount.data.length === 0);
      assert.isTrue(closedAccount.owner.equals(SystemProgram.programId));
    });

    it("Can close a mut account manually", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.methods
        .initialize(new anchor.BN(10), new anchor.BN(10))
        .accounts({ data: data.publicKey })
        .preInstructions([await program.account.data.createInstruction(data)])
        .signers([data])
        .rpc();

      const connection = program.provider.connection;
      const openAccount = await connection.getAccountInfo(data.publicKey);

      assert.isNotNull(openAccount);
      const openAccountBalance = openAccount.lamports;
      // double balance to calculate closed balance correctly
      const transferIx = anchor.web3.SystemProgram.transfer({
        fromPubkey: provider.wallet.publicKey,
        toPubkey: data.publicKey,
        lamports: openAccountBalance,
      });
      const transferTransaction = new anchor.web3.Transaction().add(transferIx);
      await provider.sendAndConfirm(transferTransaction);

      let beforeBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      await program.methods
        .testCloseMut()
        .accounts({
          data: data.publicKey,
          solDest: provider.wallet.publicKey,
        })
        .postInstructions([transferIx])
        .rpc();

      let afterBalance = (
        await connection.getAccountInfo(provider.wallet.publicKey)
      ).lamports;

      // Retrieved rent exemption sol.
      expect(afterBalance > beforeBalance).to.be.true;

      const closedAccount = await connection.getAccountInfo(data.publicKey);
      assert.isTrue(closedAccount.data.length === 0);
      assert.isTrue(closedAccount.owner.equals(SystemProgram.programId));
    });

    it("Can use instruction data in accounts constraints", async () => {
      // b"my-seed"
      const seed = Buffer.from([109, 121, 45, 115, 101, 101, 100]);
      const [myPda, nonce] = await PublicKey.findProgramAddress(
        [seed, anchor.web3.SYSVAR_RENT_PUBKEY.toBuffer()],
        program.programId
      );

      await program.rpc.testInstructionConstraint(nonce, {
        accounts: {
          myPda,
          myAccount: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
      });
    });

    it("Can create a PDA account with instruction data", async () => {
      const seed = Buffer.from([1, 2, 3, 4]);
      const domain = "my-domain";
      const foo = anchor.web3.SYSVAR_RENT_PUBKEY;
      const [myPda, nonce] = await PublicKey.findProgramAddress(
        [
          Buffer.from(anchor.utils.bytes.utf8.encode("my-seed")),
          Buffer.from(anchor.utils.bytes.utf8.encode(domain)),
          foo.toBuffer(),
          seed,
        ],
        program.programId
      );

      await program.rpc.testPdaInit(domain, seed, nonce, {
        accounts: {
          myPda,
          myPayer: provider.wallet.publicKey,
          foo,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
      });

      const myPdaAccount = await program.account.dataU16.fetch(myPda);
      assert.strictEqual(myPdaAccount.data, 6);
    });

    it("Can create a zero copy PDA account", async () => {
      const [myPda, nonce] = await PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode("my-seed"))],
        program.programId
      );
      await program.rpc.testPdaInitZeroCopy({
        accounts: {
          myPda,
          myPayer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
      });

      const myPdaAccount = await program.account.dataZeroCopy.fetch(myPda);
      assert.strictEqual(myPdaAccount.data, 9);
      assert.strictEqual(myPdaAccount.bump, nonce);
    });

    it("Can write to a zero copy PDA account", async () => {
      const [myPda, bump] = await PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode("my-seed"))],
        program.programId
      );
      await program.rpc.testPdaMutZeroCopy({
        accounts: {
          myPda,
          myPayer: provider.wallet.publicKey,
        },
      });

      const myPdaAccount = await program.account.dataZeroCopy.fetch(myPda);
      assert.strictEqual(myPdaAccount.data, 1234);
      assert.strictEqual(myPdaAccount.bump, bump);
    });

    it("Can create a token account from seeds pda", async () => {
      const [mint, mint_bump] = await PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode("my-mint-seed"))],
        program.programId
      );
      const [myPda, token_bump] = await PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode("my-token-seed"))],
        program.programId
      );
      await program.rpc.testTokenSeedsInit({
        accounts: {
          myPda,
          mint,
          authority: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      });

      const mintAccount = new Token(
        program.provider.connection,
        mint,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const account = await mintAccount.getAccountInfo(myPda);
      // @ts-expect-error
      assert.strictEqual(account.state, 1);
      assert.strictEqual(account.amount.toNumber(), 0);
      assert.isTrue(account.isInitialized);
      assert.isTrue(account.owner.equals(provider.wallet.publicKey));
      assert.isTrue(account.mint.equals(mint));
    });

    it("Can execute a fallback function", async () => {
      await nativeAssert.rejects(
        async () => {
          await anchor.utils.rpc.invoke(program.programId);
        },
        (err) => {
          assert.isTrue(err.toString().includes("custom program error: 0x4d2"));
          return true;
        }
      );
    });

    it("Can init a random account", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testInit({
        accounts: {
          data: data.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data],
      });

      const account = await program.account.dataI8.fetch(data.publicKey);
      assert.strictEqual(account.data, 3);
    });

    it("Can init a random account prefunded", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testInit({
        accounts: {
          data: data.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data],
        instructions: [
          anchor.web3.SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: data.publicKey,
            lamports: 4039280,
          }),
        ],
      });

      const account = await program.account.dataI8.fetch(data.publicKey);
      assert.strictEqual(account.data, 3);
    });

    it("Should fail when trying to init the payer as a program account", async () => {
      try {
        await program.rpc.testInit({
          accounts: {
            data: provider.wallet.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
        });
        assert.fail("Transaction should fail");
      } catch (e) {
        // "Error Code: TryingToInitPayerAsProgramAccount. Error Number: 4101. Error Message: You cannot/should not initialize the payer account as a program account."
        assert.strictEqual(e.error.errorCode.number, 4101);
      }
    });

    it("Can init a random zero copy account", async () => {
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testInitZeroCopy({
        accounts: {
          data: data.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data],
      });
      const account = await program.account.dataZeroCopy.fetch(data.publicKey);
      assert.strictEqual(account.data, 10);
      assert.strictEqual(account.bump, 2);
    });

    let mint = undefined;

    it("Can create a random mint account", async () => {
      mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });
      const client = new Token(
        program.provider.connection,
        mint.publicKey,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const mintAccount = await client.getMintInfo();
      assert.strictEqual(mintAccount.decimals, 6);
      assert.isTrue(
        mintAccount.mintAuthority.equals(provider.wallet.publicKey)
      );
      assert.isTrue(
        mintAccount.freezeAuthority.equals(provider.wallet.publicKey)
      );
    });

    it("Can create a random mint account prefunded", async () => {
      mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
        instructions: [
          anchor.web3.SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: mint.publicKey,
            lamports: 4039280,
          }),
        ],
      });
      const client = new Token(
        program.provider.connection,
        mint.publicKey,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const mintAccount = await client.getMintInfo();
      assert.strictEqual(mintAccount.decimals, 6);
      assert.isTrue(
        mintAccount.mintAuthority.equals(provider.wallet.publicKey)
      );
    });

    it("Can create a random token account", async () => {
      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
      });
      const client = new Token(
        program.provider.connection,
        mint.publicKey,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const account = await client.getAccountInfo(token.publicKey);
      // @ts-expect-error
      assert.strictEqual(account.state, 1);
      assert.strictEqual(account.amount.toNumber(), 0);
      assert.isTrue(account.isInitialized);
      assert.isTrue(account.owner.equals(provider.wallet.publicKey));
      assert.isTrue(account.mint.equals(mint.publicKey));
    });

    it("Can create a random token with prefunding", async () => {
      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
        instructions: [
          anchor.web3.SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: token.publicKey,
            lamports: 4039280,
          }),
        ],
      });
      const client = new Token(
        program.provider.connection,
        mint.publicKey,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const account = await client.getAccountInfo(token.publicKey);
      // @ts-expect-error
      assert.strictEqual(account.state, 1);
      assert.strictEqual(account.amount.toNumber(), 0);
      assert.isTrue(account.isInitialized);
      assert.isTrue(account.owner.equals(provider.wallet.publicKey));
      assert.isTrue(account.mint.equals(mint.publicKey));
    });

    it("Can create a random token with prefunding under the rent exemption", async () => {
      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
        instructions: [
          anchor.web3.SystemProgram.transfer({
            fromPubkey: provider.wallet.publicKey,
            toPubkey: token.publicKey,
            lamports: 1,
          }),
        ],
      });
      const client = new Token(
        program.provider.connection,
        mint.publicKey,
        TOKEN_PROGRAM_ID,
        wallet.payer
      );
      const account = await client.getAccountInfo(token.publicKey);
      // @ts-expect-error
      assert.strictEqual(account.state, 1);
      assert.strictEqual(account.amount.toNumber(), 0);
      assert.isTrue(account.isInitialized);
      assert.isTrue(account.owner.equals(provider.wallet.publicKey));
      assert.isTrue(account.mint.equals(mint.publicKey));
    });

    it("Can initialize multiple accounts via a composite payer", async () => {
      const data1 = anchor.web3.Keypair.generate();
      const data2 = anchor.web3.Keypair.generate();

      const tx = await program.methods
        .testCompositePayer()
        .accounts({
          composite: {
            data: data1.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          data: data2.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([data1, data2])
        .rpc();

      const account1 = await program.account.dataI8.fetch(data1.publicKey);
      assert.strictEqual(account1.data, 1);

      const account2 = await program.account.data.fetch(data2.publicKey);
      assert.strictEqual(account2.udata.toNumber(), 2);
      assert.strictEqual(account2.idata.toNumber(), 3);
    });

    describe("associated_token constraints", () => {
      let associatedToken = null;
      // apparently cannot await here so doing it in the 'it' statements
      let client = Token.createMint(
        program.provider.connection,
        wallet.payer,
        provider.wallet.publicKey,
        provider.wallet.publicKey,
        9,
        TOKEN_PROGRAM_ID
      );

      it("Can create an associated token account", async () => {
        const localClient = await client;
        associatedToken = await Token.getAssociatedTokenAddress(
          ASSOCIATED_TOKEN_PROGRAM_ID,
          TOKEN_PROGRAM_ID,
          localClient.publicKey,
          provider.wallet.publicKey
        );

        await program.rpc.testInitAssociatedToken({
          accounts: {
            token: associatedToken,
            mint: localClient.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          },
        });

        const account = await localClient.getAccountInfo(associatedToken);
        // @ts-expect-error
        assert.strictEqual(account.state, 1);
        assert.strictEqual(account.amount.toNumber(), 0);
        assert.isTrue(account.isInitialized);
        assert.isTrue(account.owner.equals(provider.wallet.publicKey));
        assert.isTrue(account.mint.equals(localClient.publicKey));
      });

      it("Can use fetchNullable() on accounts with only a balance", async () => {
        const account = anchor.web3.Keypair.generate();

        // Airdrop 1 SOL to the account.
        const signature = await program.provider.connection.requestAirdrop(
          account.publicKey,
          anchor.web3.LAMPORTS_PER_SOL
        );
        await program.provider.connection.confirmTransaction(signature);

        const data = await program.account.data.fetchNullable(
          account.publicKey
        );
        assert.isNull(data);
      });

      it("Can validate associated_token constraints", async () => {
        const localClient = await client;
        await program.rpc.testValidateAssociatedToken({
          accounts: {
            token: associatedToken,
            mint: localClient.publicKey,
            wallet: provider.wallet.publicKey,
          },
        });

        let otherMint = await Token.createMint(
          program.provider.connection,
          wallet.payer,
          provider.wallet.publicKey,
          provider.wallet.publicKey,
          9,
          TOKEN_PROGRAM_ID
        );

        await nativeAssert.rejects(
          async () => {
            await program.rpc.testValidateAssociatedToken({
              accounts: {
                token: associatedToken,
                mint: otherMint.publicKey,
                wallet: provider.wallet.publicKey,
              },
            });
          },
          (err) => {
            assert.strictEqual(err.error.errorCode.number, 2009);
            return true;
          }
        );
      });

      it("associated_token constraints check do not allow authority change", async () => {
        const localClient = await client;
        await program.rpc.testValidateAssociatedToken({
          accounts: {
            token: associatedToken,
            mint: localClient.publicKey,
            wallet: provider.wallet.publicKey,
          },
        });

        await localClient.setAuthority(
          associatedToken,
          anchor.web3.Keypair.generate().publicKey,
          "AccountOwner",
          wallet.payer,
          []
        );

        await nativeAssert.rejects(
          async () => {
            await program.rpc.testValidateAssociatedToken({
              accounts: {
                token: associatedToken,
                mint: localClient.publicKey,
                wallet: provider.wallet.publicKey,
              },
            });
          },
          (err) => {
            assert.strictEqual(err.error.errorCode.number, 2015);
            return true;
          }
        );
      });
    });

    it("Can fetch all accounts of a given type", async () => {
      // Initialize the accounts.
      const data1 = anchor.web3.Keypair.generate();
      const data2 = anchor.web3.Keypair.generate();
      const data3 = anchor.web3.Keypair.generate();
      const data4 = anchor.web3.Keypair.generate();
      // Initialize filterable data.
      const filterable1 = anchor.web3.Keypair.generate().publicKey;
      const filterable2 = anchor.web3.Keypair.generate().publicKey;
      // Set up a secondary wallet and program.
      const anotherProvider = new anchor.AnchorProvider(
        program.provider.connection,
        new anchor.Wallet(anchor.web3.Keypair.generate()),
        { commitment: program.provider.connection.commitment }
      );
      const anotherProgram = new anchor.Program(
        miscIdl,
        program.programId,
        anotherProvider
      );
      // Request airdrop for secondary wallet.
      const signature = await program.provider.connection.requestAirdrop(
        anotherProvider.wallet.publicKey,
        anchor.web3.LAMPORTS_PER_SOL
      );
      await program.provider.connection.confirmTransaction(signature);
      // Create all the accounts.
      await Promise.all([
        program.rpc.testFetchAll(filterable1, {
          accounts: {
            data: data1.publicKey,
            authority: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          signers: [data1],
        }),
        program.rpc.testFetchAll(filterable1, {
          accounts: {
            data: data2.publicKey,
            authority: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          signers: [data2],
        }),
        program.rpc.testFetchAll(filterable2, {
          accounts: {
            data: data3.publicKey,
            authority: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          signers: [data3],
        }),
        anotherProgram.rpc.testFetchAll(filterable1, {
          accounts: {
            data: data4.publicKey,
            authority: anotherProvider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
          },
          signers: [data4],
        }),
      ]);
      // Call for multiple kinds of .all.
      const allAccounts = await program.account.dataWithFilter.all();
      const allAccountsFilteredByBuffer =
        await program.account.dataWithFilter.all(
          provider.wallet.publicKey.toBuffer()
        );
      const allAccountsFilteredByProgramFilters1 =
        await program.account.dataWithFilter.all([
          {
            memcmp: {
              offset: 8,
              bytes: provider.wallet.publicKey.toBase58(),
            },
          },
          { memcmp: { offset: 40, bytes: filterable1.toBase58() } },
        ]);
      const allAccountsFilteredByProgramFilters2 =
        await program.account.dataWithFilter.all([
          {
            memcmp: {
              offset: 8,
              bytes: provider.wallet.publicKey.toBase58(),
            },
          },
          { memcmp: { offset: 40, bytes: filterable2.toBase58() } },
        ]);
      // Without filters there should be 4 accounts.
      assert.lengthOf(allAccounts, 4);
      // Filtering by main wallet there should be 3 accounts.
      assert.lengthOf(allAccountsFilteredByBuffer, 3);
      // Filtering all the main wallet accounts and matching the filterable1 value
      // results in a 2 accounts.
      assert.lengthOf(allAccountsFilteredByProgramFilters1, 2);
      // Filtering all the main wallet accounts and matching the filterable2 value
      // results in 1 account.
      assert.lengthOf(allAccountsFilteredByProgramFilters2, 1);
    });

    it("Can use pdas with empty seeds", async () => {
      const [pda, bump] = await PublicKey.findProgramAddress(
        [],
        program.programId
      );

      await program.rpc.testInitWithEmptySeeds({
        accounts: {
          pda: pda,
          authority: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
      });
      await program.rpc.testEmptySeedsConstraint({
        accounts: {
          pda: pda,
        },
      });

      const [pda2] = await PublicKey.findProgramAddress(
        [anchor.utils.bytes.utf8.encode("non-empty")],
        program.programId
      );
      await nativeAssert.rejects(
        program.rpc.testEmptySeedsConstraint({
          accounts: {
            pda: pda2,
          },
        }),
        (err) => {
          assert.equal(err.error.errorCode.number, 2006);
          return true;
        }
      );
    });

    const ifNeededAcc = anchor.web3.Keypair.generate();

    it("Can init if needed a new account", async () => {
      await program.rpc.testInitIfNeeded(1, {
        accounts: {
          data: ifNeededAcc.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: provider.wallet.publicKey,
        },
        signers: [ifNeededAcc],
      });
      const account = await program.account.dataU16.fetch(
        ifNeededAcc.publicKey
      );
      assert.strictEqual(account.data, 1);
    });

    it("Can init if needed a previously created account", async () => {
      await program.rpc.testInitIfNeeded(3, {
        accounts: {
          data: ifNeededAcc.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: provider.wallet.publicKey,
        },
        signers: [ifNeededAcc],
      });
      const account = await program.account.dataU16.fetch(
        ifNeededAcc.publicKey
      );
      assert.strictEqual(account.data, 3);
    });

    it("Can use const for array size", async () => {
      const data = anchor.web3.Keypair.generate();
      const tx = await program.rpc.testConstArraySize(99, {
        accounts: {
          data: data.publicKey,
        },
        signers: [data],
        instructions: [
          await program.account.dataConstArraySize.createInstruction(data),
        ],
      });
      const dataAccount = await program.account.dataConstArraySize.fetch(
        data.publicKey
      );
      assert.deepStrictEqual(dataAccount.data, [99, ...new Array(9).fill(0)]);
    });

    it("Can use const for instruction data size", async () => {
      const data = anchor.web3.Keypair.generate();
      const dataArray = [99, ...new Array(9).fill(0)];
      const tx = await program.rpc.testConstIxDataSize(dataArray, {
        accounts: {
          data: data.publicKey,
        },
        signers: [data],
        instructions: [
          await program.account.dataConstArraySize.createInstruction(data),
        ],
      });
      const dataAccount = await program.account.dataConstArraySize.fetch(
        data.publicKey
      );
      assert.deepStrictEqual(dataAccount.data, dataArray);
    });

    it("Should include BASE const in IDL", async () => {
      assert.isDefined(
        miscIdl.constants.find(
          (c) =>
            c.name === "BASE" && c.type === "u128" && c.value === "1_000_000"
        )
      );
    });

    it("Should include DECIMALS const in IDL", async () => {
      assert.isDefined(
        miscIdl.constants.find(
          (c) => c.name === "DECIMALS" && c.type === "u8" && c.value === "6"
        )
      );
    });

    it("Should not include NO_IDL const in IDL", async () => {
      assert.isUndefined(miscIdl.constants.find((c) => c.name === "NO_IDL"));
    });

    it("init_if_needed throws if account exists but is not owned by the expected program", async () => {
      const newAcc = await anchor.web3.PublicKey.findProgramAddress(
        [utf8.encode("hello")],
        program.programId
      );
      await program.rpc.testInitIfNeededChecksOwner({
        accounts: {
          data: newAcc[0],
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: provider.wallet.publicKey,
          owner: program.programId,
        },
      });

      try {
        await program.rpc.testInitIfNeededChecksOwner({
          accounts: {
            data: newAcc[0],
            systemProgram: anchor.web3.SystemProgram.programId,
            payer: provider.wallet.publicKey,
            owner: anchor.web3.Keypair.generate().publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2004);
      }
    });

    it("init_if_needed throws if pda account exists but does not have the expected seeds", async () => {
      const newAcc = await anchor.web3.PublicKey.findProgramAddress(
        [utf8.encode("nothello")],
        program.programId
      );
      await program.rpc.testInitIfNeededChecksSeeds("nothello", {
        accounts: {
          data: newAcc[0],
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: provider.wallet.publicKey,
        },
      });

      // this will throw if it is not a proper PDA
      // we need this so we know that the following tx failed
      // not because it couldn't create this pda
      // but because the two pdas were different
      anchor.web3.PublicKey.createProgramAddress(
        [utf8.encode("hello")],
        program.programId
      );

      try {
        await program.rpc.testInitIfNeededChecksSeeds("hello", {
          accounts: {
            data: newAcc[0],
            systemProgram: anchor.web3.SystemProgram.programId,
            payer: provider.wallet.publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2006);
      }
    });

    it("init_if_needed throws if account exists but is not the expected space", async () => {
      const newAcc = anchor.web3.Keypair.generate();
      const _irrelevantForTest = 3;
      await program.rpc.initWithSpace(_irrelevantForTest, {
        accounts: {
          data: newAcc.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: provider.wallet.publicKey,
        },
        signers: [newAcc],
      });

      try {
        await program.rpc.testInitIfNeeded(_irrelevantForTest, {
          accounts: {
            data: newAcc.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            payer: provider.wallet.publicKey,
          },
          signers: [newAcc],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2019);
      }
    });

    it("init_if_needed throws if mint exists but has the wrong mint authority", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      try {
        await program.rpc.testInitMintIfNeeded(6, {
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            mintAuthority: anchor.web3.Keypair.generate().publicKey,
            freezeAuthority: provider.wallet.publicKey,
          },
          signers: [mint],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2016);
      }
    });

    it("init_if_needed throws if mint exists but has the wrong freeze authority", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      try {
        await program.rpc.testInitMintIfNeeded(6, {
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            mintAuthority: provider.wallet.publicKey,
            freezeAuthority: anchor.web3.Keypair.generate().publicKey,
          },
          signers: [mint],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2017);
      }
    });

    it("init_if_needed throws if mint exists but has the wrong decimals", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      try {
        await program.rpc.testInitMintIfNeeded(9, {
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            mintAuthority: provider.wallet.publicKey,
            freezeAuthority: provider.wallet.publicKey,
          },
          signers: [mint],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2018);
      }
    });

    it("init_if_needed throws if token exists but has the wrong owner", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
      });

      try {
        await program.rpc.testInitTokenIfNeeded({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            authority: anchor.web3.Keypair.generate().publicKey,
          },
          signers: [token],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2015);
      }
    });

    it("init_if_needed throws if token exists but has the wrong mint", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      const mint2 = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint2.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint2],
      });

      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
      });

      try {
        await program.rpc.testInitTokenIfNeeded({
          accounts: {
            token: token.publicKey,
            mint: mint2.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            authority: provider.wallet.publicKey,
          },
          signers: [token],
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2014);
      }
    });

    it("init_if_needed throws if associated token exists but has the wrong owner", async () => {
      const mint = Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      const associatedToken = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint.publicKey,
        provider.wallet.publicKey
      );

      await program.rpc.testInitAssociatedToken({
        accounts: {
          token: associatedToken,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        },
      });

      try {
        await program.rpc.testInitAssociatedTokenIfNeeded({
          accounts: {
            token: associatedToken,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            authority: anchor.web3.Keypair.generate().publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2015);
      }
    });

    it("init_if_needed throws if associated token exists but has the wrong mint", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      const mint2 = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint2.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint2],
      });

      const associatedToken = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint.publicKey,
        provider.wallet.publicKey
      );

      const txn = await program.rpc.testInitAssociatedToken({
        accounts: {
          token: associatedToken,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        },
      });
      console.log("InitAssocToken:", txn);

      try {
        await program.rpc.testInitAssociatedTokenIfNeeded({
          accounts: {
            token: associatedToken,
            mint: mint2.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            authority: provider.wallet.publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 2014);
      }
    });

    it("init_if_needed throws if token exists with correct owner and mint but is not the ATA", async () => {
      const mint = anchor.web3.Keypair.generate();
      await program.rpc.testInitMint({
        accounts: {
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [mint],
      });

      const associatedToken = await Token.getAssociatedTokenAddress(
        ASSOCIATED_TOKEN_PROGRAM_ID,
        TOKEN_PROGRAM_ID,
        mint.publicKey,
        provider.wallet.publicKey
      );

      await program.rpc.testInitAssociatedToken({
        accounts: {
          token: associatedToken,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,

          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        },
      });

      const token = anchor.web3.Keypair.generate();
      await program.rpc.testInitToken({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        signers: [token],
      });

      try {
        await program.rpc.testInitAssociatedTokenIfNeeded({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,

            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
            authority: provider.wallet.publicKey,
          },
        });
        expect(false).to.be.true;
      } catch (_err) {
        assert.isTrue(_err instanceof AnchorError);
        const err: AnchorError = _err;
        assert.strictEqual(err.error.errorCode.number, 3014);
      }
    });

    it("Can use multidimensional array", async () => {
      const array2d = new Array(10).fill(new Array(10).fill(99));
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testMultidimensionalArray(array2d, {
        accounts: {
          data: data.publicKey,
        },
        signers: [data],
        instructions: [
          await program.account.dataMultidimensionalArray.createInstruction(
            data
          ),
        ],
      });
      const dataAccount = await program.account.dataMultidimensionalArray.fetch(
        data.publicKey
      );
      assert.deepStrictEqual(dataAccount.data, array2d);
    });

    it("Can use multidimensional array with const sizes", async () => {
      const array2d = new Array(10).fill(new Array(11).fill(22));
      const data = anchor.web3.Keypair.generate();
      await program.rpc.testMultidimensionalArrayConstSizes(array2d, {
        accounts: {
          data: data.publicKey,
        },
        signers: [data],
        instructions: [
          await program.account.dataMultidimensionalArrayConstSizes.createInstruction(
            data
          ),
        ],
      });
      const dataAccount =
        await program.account.dataMultidimensionalArrayConstSizes.fetch(
          data.publicKey
        );
      assert.deepStrictEqual(dataAccount.data, array2d);
    });

    describe("Can validate PDAs derived from other program ids", () => {
      it("With bumps using create_program_address", async () => {
        const [firstPDA, firstBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("seed")],
            ASSOCIATED_TOKEN_PROGRAM_ID
          );
        const [secondPDA, secondBump] =
          await anchor.web3.PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("seed")],
            program.programId
          );

        // correct bump but wrong address
        const wrongAddress = anchor.web3.Keypair.generate().publicKey;
        try {
          await program.rpc.testProgramIdConstraint(firstBump, secondBump, {
            accounts: {
              first: wrongAddress,
              second: secondPDA,
            },
          });
          expect(false).to.be.true;
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2006);
        }

        // matching bump seed for wrong address but derived from wrong program
        try {
          await program.rpc.testProgramIdConstraint(secondBump, secondBump, {
            accounts: {
              first: secondPDA,
              second: secondPDA,
            },
          });
          expect(false).to.be.true;
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2006);
        }

        // correct inputs should lead to successful tx
        await program.rpc.testProgramIdConstraint(firstBump, secondBump, {
          accounts: {
            first: firstPDA,
            second: secondPDA,
          },
        });
      });

      it("With bumps using find_program_address", async () => {
        const firstPDA = (
          await anchor.web3.PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("seed")],
            ASSOCIATED_TOKEN_PROGRAM_ID
          )
        )[0];
        const secondPDA = (
          await anchor.web3.PublicKey.findProgramAddress(
            [anchor.utils.bytes.utf8.encode("seed")],
            program.programId
          )
        )[0];

        // random wrong address
        const wrongAddress = anchor.web3.Keypair.generate().publicKey;
        try {
          await program.rpc.testProgramIdConstraintFindPda({
            accounts: {
              first: wrongAddress,
              second: secondPDA,
            },
          });
          expect(false).to.be.true;
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2006);
        }

        // same seeds but derived from wrong program
        try {
          await program.rpc.testProgramIdConstraintFindPda({
            accounts: {
              first: secondPDA,
              second: secondPDA,
            },
          });
          expect(false).to.be.true;
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2006);
        }

        // correct inputs should lead to successful tx
        await program.rpc.testProgramIdConstraintFindPda({
          accounts: {
            first: firstPDA,
            second: secondPDA,
          },
        });
      });
    });
    describe("Token Constraint Test", () => {
      it("Token Constraint Test(no init) - Can make token::mint and token::authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        await program.rpc.testTokenConstraint({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
          },
        });
        const mintAccount = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const account = await mintAccount.getAccountInfo(token.publicKey);
        assert.isTrue(account.owner.equals(provider.wallet.publicKey));
        assert.isTrue(account.mint.equals(mint.publicKey));
      });

      it("Token Constraint Test(no init) - Can make only token::authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        await program.rpc.testOnlyAuthConstraint({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
          },
        });
        const mintAccount = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const account = await mintAccount.getAccountInfo(token.publicKey);
        assert.isTrue(account.owner.equals(provider.wallet.publicKey));
      });

      it("Token Constraint Test(no init) - Can make only token::mint", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        await program.rpc.testOnlyMintConstraint({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
          },
        });
        const mintAccount = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const account = await mintAccount.getAccountInfo(token.publicKey);
        assert.isTrue(account.mint.equals(mint.publicKey));
      });

      it("Token Constraint Test(no init) - throws if token::mint mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const mint1 = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint1.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint1],
        });

        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        try {
          await program.rpc.testTokenConstraint({
            accounts: {
              token: token.publicKey,
              mint: mint1.publicKey,
              payer: provider.wallet.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2014);
          assert.strictEqual(err.error.errorCode.code, "ConstraintTokenMint");
        }
      });

      it("Token Constraint Test(no init) - throws if token::authority mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });
        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        const fakeAuthority = Keypair.generate();
        try {
          await program.rpc.testTokenAuthConstraint({
            accounts: {
              token: token.publicKey,
              mint: mint.publicKey,
              fakeAuthority: fakeAuthority.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2015);
          assert.strictEqual(err.error.errorCode.code, "ConstraintTokenOwner");
        }
      });

      it("Token Constraint Test(no init) - throws if both token::authority, token::mint mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });
        const mint1 = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint1.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint1],
        });
        const token = anchor.web3.Keypair.generate();
        await program.rpc.testInitToken({
          accounts: {
            token: token.publicKey,
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [token],
        });
        const fakeAuthority = Keypair.generate();
        try {
          await program.rpc.testTokenAuthConstraint({
            accounts: {
              token: token.publicKey,
              mint: mint1.publicKey,
              fakeAuthority: fakeAuthority.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2015);
          assert.strictEqual(err.error.errorCode.code, "ConstraintTokenOwner");
        }
      });

      it("Mint Constraint Test(no init) - mint::decimals, mint::authority, mint::freeze_authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });
        await program.rpc.testMintConstraint(6, {
          accounts: {
            mint: mint.publicKey,
            mintAuthority: provider.wallet.publicKey,
            freezeAuthority: provider.wallet.publicKey,
          },
        });
        const client = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const mintAccount = await client.getMintInfo();
        assert.strictEqual(mintAccount.decimals, 6);
        assert.isTrue(
          mintAccount.mintAuthority.equals(provider.wallet.publicKey)
        );
        assert.isTrue(
          mintAccount.freezeAuthority.equals(provider.wallet.publicKey)
        );
      });

      it("Mint Constraint Test(no init) - throws if mint::decimals mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });
        const fakeDecimal = 5;
        try {
          await program.rpc.testMintConstraint(fakeDecimal, {
            accounts: {
              mint: mint.publicKey,
              mintAuthority: provider.wallet.publicKey,
              freezeAuthority: provider.wallet.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2018);
          assert.strictEqual(
            err.error.errorCode.code,
            "ConstraintMintDecimals"
          );
        }
      });

      it("Mint Constraint Test(no init) - throws if mint::mint_authority mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const fakeAuthority = Keypair.generate();
        try {
          await program.rpc.testMintConstraint(6, {
            accounts: {
              mint: mint.publicKey,
              mintAuthority: fakeAuthority.publicKey,
              freezeAuthority: provider.wallet.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2016);
          assert.strictEqual(
            err.error.errorCode.code,
            "ConstraintMintMintAuthority"
          );
        }
      });

      it("Mint Constraint Test(no init) - throws if mint::freeze_authority mismatch", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        const fakeAuthority = Keypair.generate();
        try {
          await program.rpc.testMintConstraint(6, {
            accounts: {
              mint: mint.publicKey,
              mintAuthority: provider.wallet.publicKey,
              freezeAuthority: fakeAuthority.publicKey,
            },
          });
          assert.isTrue(false);
        } catch (_err) {
          assert.isTrue(_err instanceof AnchorError);
          const err: AnchorError = _err;
          assert.strictEqual(err.error.errorCode.number, 2017);
          assert.strictEqual(
            err.error.errorCode.code,
            "ConstraintMintFreezeAuthority"
          );
        }
      });

      it("Mint Constraint Test(no init) - can write only mint::decimals", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        await program.rpc.testMintOnlyDecimalsConstraint(6, {
          accounts: {
            mint: mint.publicKey,
          },
        });
        const client = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const mintAccount = await client.getMintInfo();
        assert.strictEqual(mintAccount.decimals, 6);
      });

      it("Mint Constraint Test(no init) - can write only mint::authority and mint::freeze_authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        await program.rpc.testMintOnlyAuthConstraint({
          accounts: {
            mint: mint.publicKey,
            mintAuthority: provider.wallet.publicKey,
            freezeAuthority: provider.wallet.publicKey,
          },
        });
        const client = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const mintAccount = await client.getMintInfo();
        assert.isTrue(
          mintAccount.mintAuthority.equals(provider.wallet.publicKey)
        );
        assert.isTrue(
          mintAccount.freezeAuthority.equals(provider.wallet.publicKey)
        );
      });

      it("Mint Constraint Test(no init) - can write only mint::authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        await program.rpc.testMintOnlyOneAuthConstraint({
          accounts: {
            mint: mint.publicKey,
            mintAuthority: provider.wallet.publicKey,
          },
        });
        const client = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const mintAccount = await client.getMintInfo();
        assert.isTrue(
          mintAccount.mintAuthority.equals(provider.wallet.publicKey)
        );
      });

      it("Mint Constraint Test(no init) - can write only mint::decimals and mint::freeze_authority", async () => {
        const mint = anchor.web3.Keypair.generate();
        await program.rpc.testInitMint({
          accounts: {
            mint: mint.publicKey,
            payer: provider.wallet.publicKey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
          signers: [mint],
        });

        await program.rpc.testMintMissMintAuthConstraint(6, {
          accounts: {
            mint: mint.publicKey,
            freezeAuthority: provider.wallet.publicKey,
          },
        });
        const client = new Token(
          program.provider.connection,
          mint.publicKey,
          TOKEN_PROGRAM_ID,
          wallet.payer
        );
        const mintAccount = await client.getMintInfo();
        assert.strictEqual(mintAccount.decimals, 6);
        assert.isTrue(
          mintAccount.freezeAuthority.equals(provider.wallet.publicKey)
        );
      });
      it("check versioned transaction is now available", async () => {
        let thisTx = new VersionedTransaction(
          new Message({
            header: {
              numReadonlySignedAccounts: 0,
              numReadonlyUnsignedAccounts: 0,
              numRequiredSignatures: 0,
            },
            accountKeys: [new PublicKey([0]).toString()],
            instructions: [{ accounts: [0], data: "", programIdIndex: 0 }],
            recentBlockhash: "",
          })
        );
        assert.isDefined(thisTx);
      });
      it("Can access enum variant fields using camel case without throwing a type error", async () => {
        const anotherProgram = new anchor.Program<Misc>(
          miscIdl,
          program.programId,
          provider
        );
        const enumWrappers =
          await anotherProgram.account.coolEnumWrapperAccount.all();
        for (let enumWrapper of enumWrappers) {
          // this code never gets run so just putting whatever
          console.log(enumWrapper.account.myEnum.variant2?.someSlot);
          console.log(enumWrapper.account.myEnum.variant2?.user1);
          console.log(enumWrapper.account.myEnum.variant3?.someSlot);
          console.log(enumWrapper.account.myEnum.variant3?.user2);
        }
      });
    });
  };
};

export default miscTest;

describe("misc", miscTest(anchor.workspace.Misc as Program<Misc>));

describe(
  "misc-optional",
  miscTest(anchor.workspace.MiscOptional as Program<MiscOptional>)
);
