import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { findProgramAddressSync } from '@project-serum/anchor/dist/cjs/utils/pubkey';
import { PublicKey } from '@solana/web3.js';
import assert from 'assert';
import { BpfUpgradeableState } from '../target/types/bpf_upgradeable_state';

describe('bpf_upgradeable_state', () => {
  const provider = anchor.Provider.env();
  // Configure the client to use the local cluster.
  anchor.setProvider(provider);

  const program = anchor.workspace.BpfUpgradeableState as Program<BpfUpgradeableState>;
  const programDataAddress = findProgramAddressSync(
    [program.programId.toBytes()],
    new anchor.web3.PublicKey("BPFLoaderUpgradeab1e11111111111111111111111")
  )[0];

  it('Reads ProgramData and sets field', async () => {
    const settings = anchor.web3.Keypair.generate();
    const tx = await program.rpc.setAdminSettings(new anchor.BN(500), {
      accounts: {
        authority: program.provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        programData: programDataAddress,
        program: program.programId,
        settings: settings.publicKey
      },
      signers: [settings]
    });
    assert.equal((await program.account.settings.fetch(settings.publicKey)).adminData, 500);

    console.log("Your transaction signature", tx);
  });

  it('Validates constraint on ProgramData', async () => {
    const settings = anchor.web3.Keypair.generate();
    try {
      const authority = anchor.web3.Keypair.generate();
      await provider.connection.confirmTransaction(
        await provider.connection.requestAirdrop(authority.publicKey, 10000000000),
        "confirmed"
      );
      await program.rpc.setAdminSettings(new anchor.BN(500), {
        accounts: {
          authority: authority.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          programData: programDataAddress,
          settings: settings.publicKey,
          program: program.programId,
        },
        signers: [settings, authority]
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 2003);
      assert.equal(err.msg, "A raw constraint was violated");
    }
  });

  it('Validates that account is ProgramData', async () => {
    const settings = anchor.web3.Keypair.generate();
    try {
      await program.rpc.setAdminSettings(new anchor.BN(500), {
        accounts: {
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          programData: program.programId,
          settings: settings.publicKey,
          program: program.programId,
        },
        signers: [settings]
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 3013);
      assert.equal(err.msg, "The given account is not a program data account");
    }
  });

  it('Validates that account is owned by the upgradeable bpf loader', async () => {
    const settings = anchor.web3.Keypair.generate();
    try {
      await program.rpc.setAdminSettings(new anchor.BN(500), {
        accounts: {
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          programData: program.provider.wallet.publicKey,
          settings: settings.publicKey,
          program: program.programId,
        },
        signers: [settings]
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 3007);
      assert.equal(err.msg, "The given account is not owned by the executing program");
    }
  });

  it('Deserializes UpgradableLoaderState and validates that programData is the expected account', async () => {
    const secondProgramAddress = new PublicKey("Fkv67TwmbakfZw2PoW57wYPbqNexAH6vuxpyT8vmrc3B");
    const secondProgramProgramDataAddress = findProgramAddressSync(
      [secondProgramAddress.toBytes()],
      new anchor.web3.PublicKey("BPFLoaderUpgradeab1e11111111111111111111111")
    )[0];

    const settings = anchor.web3.Keypair.generate();
    try {
      await program.rpc.setAdminSettings(new anchor.BN(500), {
        accounts: {
          authority: program.provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          programData: secondProgramProgramDataAddress,
          settings: settings.publicKey,
          program: program.programId,
        },
        signers: [settings]
      });
      assert.ok(false);
    } catch (err) {
      assert.equal(err.code, 6000);
    }
  });
});
