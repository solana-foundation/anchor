import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import Provider, { NodeWallet as Wallet } from "./provider";
import Coder from "./coder";
import { Idl } from "./idl";
import workspace from "./workspace";
import utils from "./utils";
import { Program } from "./program";
import { ProgramAccount } from "./program/namespace";
import { Context, Accounts } from "./program/context";

let _provider: Provider | null = null;

function setProvider(provider: Provider) {
  _provider = provider;
}

function getProvider(): Provider {
  if (_provider === null) {
    return Provider.local();
  }
  return _provider;
}

export {
  workspace,
  Program,
  ProgramAccount,
  Context,
  Accounts,
  Coder,
  setProvider,
  getProvider,
  Provider,
  BN,
  web3,
  Idl,
  utils,
  Wallet,
};
