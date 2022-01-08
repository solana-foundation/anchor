import * as BufferLayout from "buffer-layout";
import camelCase from "camelcase";
import { PublicKey } from "@solana/web3.js";
import { InstructionCoder } from "../index.js";
import { Idl } from "../../idl.js";

export class SplTokenInstructionCoder implements InstructionCoder {
  constructor(private idl: Idl) {}

  encode(ixName: string, ix: any): Buffer {
    switch (camelCase(ixName)) {
      case "initializeMint": {
        return encodeInitializeMint(ix);
      }
      case "initializeAccount": {
        return encodeInitializeAccount(ix);
      }
      case "initializeMultisig": {
        return encodeInitializeMultisig(ix);
      }
      case "transfer": {
        return encodeTransfer(ix);
      }
      case "approve": {
        return encodeApprove(ix);
      }
      case "revoke": {
        return encodeRevoke(ix);
      }
      case "setAuthority": {
        return encodeSetAuthority(ix);
      }
      case "mintTo": {
        return encodeMintTo(ix);
      }
      case "burn": {
        return encodeBurn(ix);
      }
      case "closeAccount": {
        return encodeCloseAccount(ix);
      }
      case "freezeAccount": {
        return encodeFreezeAccount(ix);
      }
      case "thawAccount": {
        return encodeThawAccount(ix);
      }
      case "transferChecked": {
        return encodeTransferChecked(ix);
      }
      case "approvedChecked": {
        return encodeApproveChecked(ix);
      }
      case "mintToChecked": {
        return encodeMintToChecked(ix);
      }
      case "burnChecked": {
        return encodeBurnChecked(ix);
      }
      case "intializeAccount2": {
        return encodeInitializeAccount2(ix);
      }
      case "syncNative": {
        return encodeSyncNative(ix);
      }
      case "initializeAccount3": {
        return encodeInitializeAccount3(ix);
      }
      case "initializeMultisig2": {
        return encodeInitializeMultisig2(ix);
      }
      case "initializeMint2": {
        return encodeInitializeMint2(ix);
      }
      default: {
        throw new Error(`Invalid instruction: ${ixName}`);
      }
    }
  }

  encodeState(_ixName: string, _ix: any): Buffer {
    throw new Error("SPL token does not have state");
  }
}

function encodeInitializeMint({
  decimals,
  mintAuthority,
  freezeAuthority,
}: any): Buffer {
  return encodeData({
    initializeMint: {
      decimals,
      mintAuthority: mintAuthority.toBuffer(),
      freezeAuthorityOption: !!freezeAuthority,
      freezeAuthority: (freezeAuthority || PublicKey.default).toBuffer(),
    },
  });
}

function encodeInitializeAccount(_ix: any): Buffer {
  return encodeData({
    initializeAccount: {},
  });
}

function encodeInitializeMultisig({ m }: any): Buffer {
  return encodeData({
    initializeMultisig: {
      m,
    },
  });
}

function encodeTransfer({ amount }: any): Buffer {
  return encodeData({
    transfer: { amount },
  });
}

function encodeApprove({ amount }: any): Buffer {
  return encodeData({
    approve: { amount },
  });
}

function encodeRevoke(_ix: any): Buffer {
  return encodeData({
    revoke: {},
  });
}

function encodeSetAuthority({ authorityType, newAuthority }: any): Buffer {
  return encodeData({
    setAuthority: { authorityType, newAuthority },
  });
}

function encodeMintTo({ amount }: any): Buffer {
  return encodeData({
    mintTo: { amount },
  });
}

function encodeBurn({ amount }: any): Buffer {
  return encodeData({
    burn: { amount },
  });
}

function encodeCloseAccount(_: any): Buffer {
  return encodeData({
    closeAccount: {},
  });
}

function encodeFreezeAccount(_: any): Buffer {
  return encodeData({
    freezeAccount: {},
  });
}

function encodeThawAccount(_: any): Buffer {
  return encodeData({
    thawAccount: {},
  });
}

function encodeTransferChecked({ amount, decimals }: any): Buffer {
  return encodeData({
    transferChecked: { amount, decimals },
  });
}

function encodeApproveChecked({ amount, decimals }: any): Buffer {
  return encodeData({
    approveChecked: { amount, decimals },
  });
}

function encodeMintToChecked({ amount, decimals }: any): Buffer {
  return encodeData({
    mintToChecked: { amount, decimals },
  });
}

function encodeBurnChecked({ amount, decimals }: any): Buffer {
  return encodeData({
    burnChecked: { amount, decimals },
  });
}

function encodeInitializeAccount2({ authority }: any): Buffer {
  return encodeData({
    initilaizeAccount2: { authority },
  });
}

function encodeSyncNative(_: any): Buffer {
  return encodeData({
    syncNative: {},
  });
}

function encodeInitializeAccount3({ authority }: any): Buffer {
  return encodeData({
    initializeAccount3: { authority },
  });
}

function encodeInitializeMultisig2({ m }: any): Buffer {
  return encodeData({
    initializeMultisig2: { m },
  });
}

function encodeInitializeMint2({
  decimals,
  mintAuthority,
  freezeAuthority,
}: any): Buffer {
  return encodeData({
    encodeInitializeMint2: { decimals, mintAuthority, freezeAuthority },
  });
}

export const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);

const LAYOUT = BufferLayout.union(BufferLayout.u8("instruction"));
LAYOUT.addVariant(
  0,
  BufferLayout.struct([
    BufferLayout.u8("decimals"),
    BufferLayout.blob(32, "mintAuthority"),
    BufferLayout.u8("freezeAuthorityOption"),
    BufferLayout.blob(32, "freezeAuthority"),
  ]),
  "initializeMint"
);
LAYOUT.addVariant(1, BufferLayout.struct([]), "initializeAccount");
LAYOUT.addVariant(
  2,
  BufferLayout.struct([BufferLayout.u8("m")]),
  "initializeMultisig"
);
LAYOUT.addVariant(
  3,
  BufferLayout.struct([BufferLayout.nu64("amount")]),
  "transfer"
);
LAYOUT.addVariant(
  7,
  BufferLayout.struct([BufferLayout.nu64("amount")]),
  "mintTo"
);
LAYOUT.addVariant(
  8,
  BufferLayout.struct([BufferLayout.nu64("amount")]),
  "burn"
);
LAYOUT.addVariant(9, BufferLayout.struct([]), "closeAccount");
LAYOUT.addVariant(
  12,
  BufferLayout.struct([
    BufferLayout.nu64("amount"),
    BufferLayout.u8("decimals"),
  ]),
  "transferChecked"
);

function encodeData(instruction: any) {
  let b = Buffer.alloc(instructionMaxSpan);
  let span = LAYOUT.encode(instruction, b);
  return b.slice(0, span);
}

const instructionMaxSpan = Math.max(
  // @ts-ignore
  ...Object.values(LAYOUT.registry).map((r) => r.span)
);
