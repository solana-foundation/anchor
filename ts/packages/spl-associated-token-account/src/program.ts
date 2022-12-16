import { PublicKey } from "@solana/web3.js";
import { Program, AnchorProvider } from "@coral-xyz/anchor";

import { SplAssociatedTokenAccountCoder } from "./coder";

export const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);

interface GetProgramParams {
  programId?: PublicKey;
  provider?: AnchorProvider;
}

export function splAssociatedTokenAccountProgram(
  params?: GetProgramParams
): Program<SplAssociatedTokenAccount> {
  return new Program<SplAssociatedTokenAccount>(
    IDL,
    params?.programId ?? SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
    params?.provider,
    new SplAssociatedTokenAccountCoder(IDL)
  );
}

type SplAssociatedTokenAccount = {
  version: "1.1.1";
  name: "spl_associated_token_account";
  instructions: [
    {
      name: "create";
      accounts: [
        {
          name: "fundingAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "associatedAccountAddress";
          isMut: true;
          isSigner: false;
        },
        {
          name: "walletAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenMintAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "createIdempotent";
      accounts: [
        {
          name: "fundingAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "associatedAccountAddress";
          isMut: true;
          isSigner: false;
        },
        {
          name: "walletAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenMintAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    },
    {
      name: "recoverNested";
      accounts: [
        {
          name: "nestedAssociatedAccountAddress";
          isMut: true;
          isSigner: false;
        },
        {
          name: "nestedTokenMintAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "destinationAssociatedAccountAddress";
          isMut: true;
          isSigner: false;
        },
        {
          name: "ownerAssociatedAccountAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "ownerTokenMintAddress";
          isMut: false;
          isSigner: false;
        },
        {
          name: "walletAddress";
          isMut: true;
          isSigner: true;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        }
      ];
      args: [];
    }
  ];
  errors: [
    {
      code: 0;
      name: "InvalidOwner";
      msg: "Associated token account owner does not match address derivation";
    }
  ];
};

const IDL: SplAssociatedTokenAccount = {
  version: "1.1.1",
  name: "spl_associated_token_account",
  instructions: [
    {
      name: "create",
      accounts: [
        {
          name: "fundingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "associatedAccountAddress",
          isMut: true,
          isSigner: false,
        },
        {
          name: "walletAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenMintAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "createIdempotent",
      accounts: [
        {
          name: "fundingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "associatedAccountAddress",
          isMut: true,
          isSigner: false,
        },
        {
          name: "walletAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenMintAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "recoverNested",
      accounts: [
        {
          name: "nestedAssociatedAccountAddress",
          isMut: true,
          isSigner: false,
        },
        {
          name: "nestedTokenMintAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "destinationAssociatedAccountAddress",
          isMut: true,
          isSigner: false,
        },
        {
          name: "ownerAssociatedAccountAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "ownerTokenMintAddress",
          isMut: false,
          isSigner: false,
        },
        {
          name: "walletAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
  ],
  errors: [
    {
      code: 0,
      name: "InvalidOwner",
      msg: "Associated token account owner does not match address derivation",
    },
  ],
};
