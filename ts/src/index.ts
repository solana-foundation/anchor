import BN from "bn.js";
import * as web3 from "@solana/web3.js";
import { Provider } from "@project-serum/common";
import { Program } from "./program";
import Coder from "./coder";
import workspace from './workspace';

let _provider: Provider | null = null;

function setProvider(provider: Provider) {
  _provider = provider;
}

function getProvider(): Provider {
  return _provider;
}

export { workspace, Program, Coder, setProvider, getProvider, Provider, BN, web3 };
