const anchor = require("@project-serum/anchor");
const PublicKey = anchor.web3.PublicKey;
const assert = require("assert");
const {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  Token,
} = require("@solana/spl-token");
const miscIdl = require("../target/idl/misc.json");
const utf8 = anchor.utils.bytes.utf8;

describe("misc", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.Misc;
  const misc2Program = anchor.workspace.Misc2;

  it("Can allocate extra space for a state constructor", async () => {
    const tx = await program.state.rpc.new();
    const addr = await program.state.address();
    const state = await program.state.fetch();
    const accountInfo = await program.provider.connection.getAccountInfo(addr);
    assert.ok(state.v.equals(Buffer.from([])));
    assert.ok(accountInfo.data.length === 99);
  });

  it("Can use remaining accounts for a state instruction", async () => {
    await program.state.rpc.remainingAccounts({
      remainingAccounts: [
        { pubkey: misc2Program.programId, isWritable: false, isSigner: false },
      ],
    });
  });

  const data = anchor.web3.Keypair.generate();

  it("Can use u128 and i128", async () => {
    const tx = await program.rpc.initialize(
      new anchor.BN(1234),
      new anchor.BN(22),
      {
        accounts: {
          data: data.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        signers: [data],
        instructions: [await program.account.data.createInstruction(data)],
      }
    );
    const dataAccount = await program.account.data.fetch(data.publicKey);
    assert.ok(dataAccount.udata.eq(new anchor.BN(1234)));
    assert.ok(dataAccount.idata.eq(new anchor.BN(22)));
  });

  it("Can use u16", async () => {
    const data = anchor.web3.Keypair.generate();
    const tx = await program.rpc.testU16(99, {
      accounts: {
        myAccount: data.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [data],
      instructions: [await program.account.dataU16.createInstruction(data)],
    });
    const dataAccount = await program.account.dataU16.fetch(data.publicKey);
    assert.ok(dataAccount.data === 99);
  });

  it("Can embed programs into genesis from the Anchor.toml", async () => {
    const pid = new anchor.web3.PublicKey(
      "FtMNMKp9DZHKWUyVAsj3Q5QV8ow4P3fUPP7ZrWEQJzKr"
    );
    let accInfo = await anchor.getProvider().connection.getAccountInfo(pid);
    assert.ok(accInfo.executable);
  });

  it("Can use the owner constraint", async () => {
    await program.rpc.testOwner({
      accounts: {
        data: data.publicKey,
        misc: program.programId,
      },
    });

    await assert.rejects(
      async () => {
        await program.rpc.testOwner({
          accounts: {
            data: program.provider.wallet.publicKey,
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

    await assert.rejects(
      async () => {
        await program.rpc.testExecutable({
          accounts: {
            program: program.provider.wallet.publicKey,
          },
        });
      },
      (err) => {
        return true;
      }
    );
  });

  it("Can CPI to state instructions", async () => {
    const oldData = new anchor.BN(0);
    await misc2Program.state.rpc.new({
      accounts: {
        authority: program.provider.wallet.publicKey,
      },
    });
    let stateAccount = await misc2Program.state.fetch();
    assert.ok(stateAccount.data.eq(oldData));
    assert.ok(stateAccount.auth.equals(program.provider.wallet.publicKey));
    const newData = new anchor.BN(2134);
    await program.rpc.testStateCpi(newData, {
      accounts: {
        authority: program.provider.wallet.publicKey,
        cpiState: await misc2Program.state.address(),
        misc2Program: misc2Program.programId,
      },
    });
    stateAccount = await misc2Program.state.fetch();
    assert.ok(stateAccount.data.eq(newData));
    assert.ok(stateAccount.auth.equals(program.provider.wallet.publicKey));
  });

  it("Can retrieve events when simulating a transaction", async () => {
    const resp = await program.simulate.testSimulate(44);
    const expectedRaw = [
      "Program Z2Ddx1Lcd8CHTV9tkWtNnFQrSz6kxz2H38wrr18zZRZ invoke [1]",
      "Program log: NgyCA9omwbMsAAAA",
      "Program log: fPhuIELK/k7SBAAA",
      "Program log: jvbowsvlmkcJAAAA",
      "Program Z2Ddx1Lcd8CHTV9tkWtNnFQrSz6kxz2H38wrr18zZRZ consumed 4819 of 200000 compute units",
      "Program Z2Ddx1Lcd8CHTV9tkWtNnFQrSz6kxz2H38wrr18zZRZ success",
    ];

    assert.ok(JSON.stringify(expectedRaw), resp.raw);
    assert.ok(resp.events[0].name === "E1");
    assert.ok(resp.events[0].data.data === 44);
    assert.ok(resp.events[1].name === "E2");
    assert.ok(resp.events[1].data.data === 1234);
    assert.ok(resp.events[2].name === "E3");
    assert.ok(resp.events[2].data.data === 9);
  });

  let dataI8;

  it("Can use i8 in the idl", async () => {
    dataI8 = anchor.web3.Keypair.generate();
    await program.rpc.testI8(-3, {
      accounts: {
        data: dataI8.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [await program.account.dataI8.createInstruction(dataI8)],
      signers: [dataI8],
    });
    const dataAccount = await program.account.dataI8.fetch(dataI8.publicKey);
    assert.ok(dataAccount.data === -3);
  });

  let dataPubkey;

  it("Can use i16 in the idl", async () => {
    const data = anchor.web3.Keypair.generate();
    await program.rpc.testI16(-2048, {
      accounts: {
        data: data.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      instructions: [await program.account.dataI16.createInstruction(data)],
      signers: [data],
    });
    const dataAccount = await program.account.dataI16.fetch(data.publicKey);
    assert.ok(dataAccount.data === -2048);

    dataPubkey = data.publicKey;
  });

  it("Can use base58 strings to fetch an account", async () => {
    const dataAccount = await program.account.dataI16.fetch(
      dataPubkey.toString()
    );
    assert.ok(dataAccount.data === -2048);
  });

  it("Should fail to close an account when sending lamports to itself", async () => {
    try {
      await program.rpc.testClose({
        accounts: {
          data: data.publicKey,
          solDest: data.publicKey,
        },
      });
      assert.ok(false);
    } catch (err) {
      const errMsg = "A close constraint was violated";
      assert.equal(err.toString(), errMsg);
      assert.equal(err.msg, errMsg);
      assert.equal(err.code, 2011);
    }
  });

  it("Can close an account", async () => {
    const openAccount = await program.provider.connection.getAccountInfo(
      data.publicKey
    );
    assert.ok(openAccount !== null);

    let beforeBalance = (
      await program.provider.connection.getAccountInfo(
        program.provider.wallet.publicKey
      )
    ).lamports;

    await program.rpc.testClose({
      accounts: {
        data: data.publicKey,
        solDest: program.provider.wallet.publicKey,
      },
    });

    let afterBalance = (
      await program.provider.connection.getAccountInfo(
        program.provider.wallet.publicKey
      )
    ).lamports;

    // Retrieved rent exemption sol.
    assert.ok(afterBalance > beforeBalance);

    const closedAccount = await program.provider.connection.getAccountInfo(
      data.publicKey
    );
    assert.ok(closedAccount === null);
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
        myPayer: program.provider.wallet.publicKey,
        foo,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
    });

    const myPdaAccount = await program.account.dataU16.fetch(myPda);
    assert.ok(myPdaAccount.data === 6);
  });

  it("Can create a zero copy PDA account", async () => {
    const [myPda, nonce] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("my-seed"))],
      program.programId
    );
    await program.rpc.testPdaInitZeroCopy(nonce, {
      accounts: {
        myPda,
        myPayer: program.provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
    });

    const myPdaAccount = await program.account.dataZeroCopy.fetch(myPda);
    assert.ok(myPdaAccount.data === 9);
    assert.ok((myPdaAccount.bump = nonce));
  });

  it("Can write to a zero copy PDA account", async () => {
    const [myPda, bump] = await PublicKey.findProgramAddress(
      [Buffer.from(anchor.utils.bytes.utf8.encode("my-seed"))],
      program.programId
    );
    await program.rpc.testPdaMutZeroCopy({
      accounts: {
        myPda,
        myPayer: program.provider.wallet.publicKey,
      },
    });

    const myPdaAccount = await program.account.dataZeroCopy.fetch(myPda);
    assert.ok(myPdaAccount.data === 1234);
    assert.ok((myPdaAccount.bump = bump));
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
    await program.rpc.testTokenSeedsInit(token_bump, mint_bump, {
      accounts: {
        myPda,
        mint,
        authority: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    const mintAccount = new Token(
      program.provider.connection,
      mint,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const account = await mintAccount.getAccountInfo(myPda);
    assert.ok(account.state === 1);
    assert.ok(account.amount.toNumber() === 0);
    assert.ok(account.isInitialized);
    assert.ok(account.owner.equals(program.provider.wallet.publicKey));
    assert.ok(account.mint.equals(mint));
  });

  it("Can execute a fallback function", async () => {
    await assert.rejects(
      async () => {
        await anchor.utils.rpc.invoke(program.programId);
      },
      (err) => {
        assert.ok(err.toString().includes("custom program error: 0x4d2"));
        return true;
      }
    );
  });

  it("Can init a random account", async () => {
    const data = anchor.web3.Keypair.generate();
    await program.rpc.testInit({
      accounts: {
        data: data.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [data],
    });

    const account = await program.account.dataI8.fetch(data.publicKey);
    assert.ok(account.data === 3);
  });

  it("Can init a random account prefunded", async () => {
    const data = anchor.web3.Keypair.generate();
    await program.rpc.testInit({
      accounts: {
        data: data.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [data],
      instructions: [
        anchor.web3.SystemProgram.transfer({
          fromPubkey: program.provider.wallet.publicKey,
          toPubkey: data.publicKey,
          lamports: 4039280,
        }),
      ],
    });

    const account = await program.account.dataI8.fetch(data.publicKey);
    assert.ok(account.data === 3);
  });

  it("Can init a random zero copy account", async () => {
    const data = anchor.web3.Keypair.generate();
    await program.rpc.testInitZeroCopy({
      accounts: {
        data: data.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [data],
    });
    const account = await program.account.dataZeroCopy.fetch(data.publicKey);
    assert.ok(account.data === 10);
    assert.ok(account.bump === 2);
  });

  let mint = undefined;

  it("Can create a random mint account", async () => {
    mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const mintAccount = await client.getMintInfo();
    assert.ok(mintAccount.decimals === 6);
    assert.ok(
      mintAccount.mintAuthority.equals(program.provider.wallet.publicKey)
    );
    assert.ok(
      mintAccount.freezeAuthority.equals(program.provider.wallet.publicKey)
    );
  });

  it("Can create a random mint account prefunded", async () => {
    mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
      instructions: [
        anchor.web3.SystemProgram.transfer({
          fromPubkey: program.provider.wallet.publicKey,
          toPubkey: mint.publicKey,
          lamports: 4039280,
        }),
      ],
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const mintAccount = await client.getMintInfo();
    assert.ok(mintAccount.decimals === 6);
    assert.ok(
      mintAccount.mintAuthority.equals(program.provider.wallet.publicKey)
    );
  });

  it("Can create a random token account", async () => {
    const token = anchor.web3.Keypair.generate();
    await program.rpc.testInitToken({
      accounts: {
        token: token.publicKey,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [token],
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const account = await client.getAccountInfo(token.publicKey);
    assert.ok(account.state === 1);
    assert.ok(account.amount.toNumber() === 0);
    assert.ok(account.isInitialized);
    assert.ok(account.owner.equals(program.provider.wallet.publicKey));
    assert.ok(account.mint.equals(mint.publicKey));
  });

  it("Can create a random token with prefunding", async () => {
    const token = anchor.web3.Keypair.generate();
    await program.rpc.testInitToken({
      accounts: {
        token: token.publicKey,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [token],
      instructions: [
        anchor.web3.SystemProgram.transfer({
          fromPubkey: program.provider.wallet.publicKey,
          toPubkey: token.publicKey,
          lamports: 4039280,
        }),
      ],
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const account = await client.getAccountInfo(token.publicKey);
    assert.ok(account.state === 1);
    assert.ok(account.amount.toNumber() === 0);
    assert.ok(account.isInitialized);
    assert.ok(account.owner.equals(program.provider.wallet.publicKey));
    assert.ok(account.mint.equals(mint.publicKey));
  });

  it("Can create a random token with prefunding under the rent exemption", async () => {
    const token = anchor.web3.Keypair.generate();
    await program.rpc.testInitToken({
      accounts: {
        token: token.publicKey,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [token],
      instructions: [
        anchor.web3.SystemProgram.transfer({
          fromPubkey: program.provider.wallet.publicKey,
          toPubkey: token.publicKey,
          lamports: 1,
        }),
      ],
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const account = await client.getAccountInfo(token.publicKey);
    assert.ok(account.state === 1);
    assert.ok(account.amount.toNumber() === 0);
    assert.ok(account.isInitialized);
    assert.ok(account.owner.equals(program.provider.wallet.publicKey));
    assert.ok(account.mint.equals(mint.publicKey));
  });

  it("Can initialize multiple accounts via a composite payer", async () => {
    const data1 = anchor.web3.Keypair.generate();
    const data2 = anchor.web3.Keypair.generate();

    const tx = await program.rpc.testCompositePayer({
      accounts: {
        composite: {
          data: data1.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        data: data2.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
      signers: [data1, data2],
    });

    const account1 = await program.account.dataI8.fetch(data1.publicKey);
    assert.equal(account1.data, 1);

    const account2 = await program.account.data.fetch(data2.publicKey);
    assert.equal(account2.udata, 2);
    assert.equal(account2.idata, 3);
  });

  let associatedToken = null;

  it("Can create an associated token account", async () => {
    associatedToken = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      program.provider.wallet.publicKey
    );

    await program.rpc.testInitAssociatedToken({
      accounts: {
        token: associatedToken,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      },
    });
    const client = new Token(
      program.provider.connection,
      mint.publicKey,
      TOKEN_PROGRAM_ID,
      program.provider.wallet.payer
    );
    const account = await client.getAccountInfo(associatedToken);
    assert.ok(account.state === 1);
    assert.ok(account.amount.toNumber() === 0);
    assert.ok(account.isInitialized);
    assert.ok(account.owner.equals(program.provider.wallet.publicKey));
    assert.ok(account.mint.equals(mint.publicKey));
  });

  it("Can validate associated_token constraints", async () => {
    await program.rpc.testValidateAssociatedToken({
      accounts: {
        token: associatedToken,
        mint: mint.publicKey,
        wallet: program.provider.wallet.publicKey,
      },
    });

    await assert.rejects(
      async () => {
        await program.rpc.testValidateAssociatedToken({
          accounts: {
            token: associatedToken,
            mint: mint.publicKey,
            wallet: anchor.web3.Keypair.generate().publicKey,
          },
        });
      },
      (err) => {
        assert.equal(err.code, 2009);
        return true;
      }
    );
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
    const anotherProgram = new anchor.Program(
      miscIdl,
      program.programId,
      new anchor.Provider(
        program.provider.connection,
        new anchor.Wallet(anchor.web3.Keypair.generate()),
        { commitment: program.provider.connection.commitment }
      )
    );
    // Request airdrop for secondary wallet.
    const signature = await program.provider.connection.requestAirdrop(
      anotherProgram.provider.wallet.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    await program.provider.connection.confirmTransaction(signature);
    // Create all the accounts.
    await Promise.all([
      program.rpc.testFetchAll(filterable1, {
        accounts: {
          data: data1.publicKey,
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data1],
      }),
      program.rpc.testFetchAll(filterable1, {
        accounts: {
          data: data2.publicKey,
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data2],
      }),
      program.rpc.testFetchAll(filterable2, {
        accounts: {
          data: data3.publicKey,
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data3],
      }),
      anotherProgram.rpc.testFetchAll(filterable1, {
        accounts: {
          data: data4.publicKey,
          authority: anotherProgram.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
        },
        signers: [data4],
      }),
    ]);
    // Call for multiple kinds of .all.
    const allAccounts = await program.account.dataWithFilter.all();
    const allAccountsFilteredByBuffer = await program.account.dataWithFilter.all(
      program.provider.wallet.publicKey.toBuffer()
    );
    const allAccountsFilteredByProgramFilters1 = await program.account.dataWithFilter.all(
      [
        {
          memcmp: {
            offset: 8,
            bytes: program.provider.wallet.publicKey.toBase58(),
          },
        },
        { memcmp: { offset: 40, bytes: filterable1.toBase58() } },
      ]
    );
    const allAccountsFilteredByProgramFilters2 = await program.account.dataWithFilter.all(
      [
        {
          memcmp: {
            offset: 8,
            bytes: program.provider.wallet.publicKey.toBase58(),
          },
        },
        { memcmp: { offset: 40, bytes: filterable2.toBase58() } },
      ]
    );
    // Without filters there should be 4 accounts.
    assert.equal(allAccounts.length, 4);
    // Filtering by main wallet there should be 3 accounts.
    assert.equal(allAccountsFilteredByBuffer.length, 3);
    // Filtering all the main wallet accounts and matching the filterable1 value
    // results in a 2 accounts.
    assert.equal(allAccountsFilteredByProgramFilters1.length, 2);
    // Filtering all the main wallet accounts and matching the filterable2 value
    // results in 1 account.
    assert.equal(allAccountsFilteredByProgramFilters2.length, 1);
  });

  it("Can use pdas with empty seeds", async () => {
    const [pda, bump] = await PublicKey.findProgramAddress(
      [],
      program.programId
    );

    await program.rpc.testInitWithEmptySeeds({
      accounts: {
        pda: pda,
        authority: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
      },
    });
    await program.rpc.testEmptySeedsConstraint({
      accounts: {
        pda: pda,
      },
    });

    const [pda2, bump2] = await PublicKey.findProgramAddress(
      ["non-empty"],
      program.programId
    );
    await assert.rejects(
      program.rpc.testEmptySeedsConstraint({
        accounts: {
          pda: pda2,
        },
      }),
      (err) => {
        assert.equal(err.code, 2006);
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
        payer: program.provider.wallet.publicKey,
      },
      signers: [ifNeededAcc],
    });
    const account = await program.account.dataU16.fetch(ifNeededAcc.publicKey);
    assert.ok(account.data, 1);
  });

  it("Can init if needed a previously created account", async () => {
    await program.rpc.testInitIfNeeded(3, {
      accounts: {
        data: ifNeededAcc.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: program.provider.wallet.publicKey,
      },
      signers: [ifNeededAcc],
    });
    const account = await program.account.dataU16.fetch(ifNeededAcc.publicKey);
    assert.ok(account.data, 3);
  });

  it("Should include BASE const in IDL", async () => {
    assert(
      miscIdl.constants.find(
        (c) => c.name === "BASE" && c.type === "u128" && c.value === "1_000_000"
      ) !== undefined
    );
  });

  it("Should include DECIMALS const in IDL", async () => {
    assert(
      miscIdl.constants.find(
        (c) => c.name === "DECIMALS" && c.type === "u8" && c.value === "6"
      ) !== undefined
    );
  });

  it("Should not include NO_IDL const in IDL", async () => {
    assert.equal(
      miscIdl.constants.find((c) => c.name === "NO_IDL"),
      undefined
    );
  });

  it("init_if_needed throws if account exists but is not owned by the expected program", async () => {
    const newAcc = await anchor.web3.PublicKey.findProgramAddress([utf8.encode("hello")], program.programId);
    await program.rpc.testInitIfNeededChecksOwner({
      accounts: {
        data: newAcc[0],
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: program.provider.wallet.publicKey,
        owner: program.programId
      }
    });

    try {
      await program.rpc.testInitIfNeededChecksOwner({
        accounts: {
          data: newAcc[0],
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: program.provider.wallet.publicKey,
          owner: anchor.web3.Keypair.generate().publicKey
        },
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2004);
    }
  });

  it("init_if_needed throws if pda account exists but does not have the expected seeds", async () => {
    const newAcc = await anchor.web3.PublicKey.findProgramAddress([utf8.encode("nothello")], program.programId);
    await program.rpc.testInitIfNeededChecksSeeds("nothello", {
      accounts: {
        data: newAcc[0],
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: program.provider.wallet.publicKey,
      }
    });

    // this will throw if it is not a proper PDA
    // we need this so we know that the following tx failed
    // not because it couldn't create this pda
    // but because the two pdas were different
    anchor.web3.PublicKey.createProgramAddress([utf8.encode("hello")], program.programId);

    try {
      await program.rpc.testInitIfNeededChecksSeeds("hello", {
        accounts: {
          data: newAcc[0],
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: program.provider.wallet.publicKey,
          owner: anchor.web3.Keypair.generate().publicKey
        },
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2006);
    }
  });



  it("init_if_needed throws if account exists but is not the expected space", async () => {
    const newAcc = anchor.web3.Keypair.generate();
    await program.rpc.initWithSpace(3, {
      accounts: {
        data: newAcc.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        payer: program.provider.wallet.publicKey,
      },
      signers: [newAcc],
    });

    try {
      await program.rpc.testInitIfNeeded(3, {
        accounts: {
          data: newAcc.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          payer: program.provider.wallet.publicKey,
        },
        signers: [newAcc],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2019);
    }
  });

  it("init_if_needed throws if mint exists but has the wrong mint authority", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    try {
      await program.rpc.testInitMintIfNeeded(6,{
        accounts: {
          mint: mint.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          mintAuthority: anchor.web3.Keypair.generate().publicKey,
          freezeAuthority: program.provider.wallet.publicKey
        },
        signers: [mint],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2016);
    }
  });

  it("init_if_needed throws if mint exists but has the wrong freeze authority", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    try {
      await program.rpc.testInitMintIfNeeded(6,{
        accounts: {
          mint: mint.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          mintAuthority: program.provider.wallet.publicKey,
          freezeAuthority: anchor.web3.Keypair.generate().publicKey
        },
        signers: [mint],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2017);
    }
  });

  it("init_if_needed throws if mint exists but has the wrong decimals", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    try {
      await program.rpc.testInitMintIfNeeded(9,{
        accounts: {
          mint: mint.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          mintAuthority: program.provider.wallet.publicKey,
          freezeAuthority: program.provider.wallet.publicKey
        },
        signers: [mint],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2018);
    }
  });

  it("init_if_needed throws if token exists but has the wrong owner", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    const token = anchor.web3.Keypair.generate();
    await program.rpc.testInitToken({
      accounts: {
        token: token.publicKey,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [token],
    });

    try {
      await program.rpc.testInitTokenIfNeeded({
        accounts: {
          token: token.publicKey,
          mint: mint.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          authority: anchor.web3.Keypair.generate().publicKey,
        },
        signers: [token],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2015);
    }
  });

  it("init_if_needed throws if token exists but has the wrong mint", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    const mint2 = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint2.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint2],
    });

    const token = anchor.web3.Keypair.generate();
    await program.rpc.testInitToken({
      accounts: {
        token: token.publicKey,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [token],
    });

    try {
      await program.rpc.testInitTokenIfNeeded({
        accounts: {
          token: token.publicKey,
          mint: mint2.publicKey,
          payer: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          authority: program.provider.wallet.publicKey,
        },
        signers: [token],
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2014);
    }
  });

  it("init_if_needed throws if associated token exists but has the wrong owner", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    const associatedToken = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      program.provider.wallet.publicKey
    );

    await program.rpc.testInitAssociatedToken({
      accounts: {
        token: associatedToken,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
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
          payer: program.provider.wallet.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          authority: anchor.web3.Keypair.generate().publicKey
        },
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2015);
    }
  })

  it("init_if_needed throws if associated token exists but has the wrong mint", async () => {
    const mint = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint],
    });

    const mint2 = anchor.web3.Keypair.generate();
    await program.rpc.testInitMint({
      accounts: {
        mint: mint2.publicKey,
        payer: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [mint2],
    });

    const associatedToken = await Token.getAssociatedTokenAddress(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      program.provider.wallet.publicKey
    );

    await program.rpc.testInitAssociatedToken({
      accounts: {
        token: associatedToken,
        mint: mint.publicKey,
        payer: program.provider.wallet.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      },
    });

    try {
      await program.rpc.testInitAssociatedTokenIfNeeded({
        accounts: {
          token: associatedToken,
          mint: mint2.publicKey,
          payer: program.provider.wallet.publicKey,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          authority: program.provider.wallet.publicKey
        },
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2014);
    }
  })

  it("Can use multidimensional array", async () => {
    const array2d = new Array(10).fill(new Array(10).fill(99));
    const data = anchor.web3.Keypair.generate();
    const tx = await program.rpc.testMultidimensionalArray(array2d, {
      accounts: {
        data: data.publicKey,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
      signers: [data],
      instructions: [
        await program.account.dataMultidimensionalArray.createInstruction(data),
      ],
    });
    const dataAccount = await program.account.dataMultidimensionalArray.fetch(
      data.publicKey
    );
    assert.deepStrictEqual(dataAccount.data, array2d);
  });
});
