import { Program } from "../index.js";
import { program as tokenProgram, SplToken } from "./token.js";

export { SplToken } from "./token.js";

export class Spl {
  public static token(): Program<SplToken> {
    return tokenProgram();
  }
}
