import { Program } from "./program";
import Coder from "./coder";
import { Provider } from "@project-serum/common";

let _provider: Provider | null = null;

function setProvider(provider: Provider) {
  _provider = provider;
}

function getProvider(): Provider {
  return _provider;
}

export { Program, Coder, setProvider, getProvider, Provider };
