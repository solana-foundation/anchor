import * as fs from "fs/promises";
import path from "path";

import { IDL } from "../target/types/bench";
import { BenchData, BinarySize } from "../scripts/utils";

describe("Binary size", () => {
  const binarySize: BinarySize = {};

  it("Measure binary size", async () => {
    const stat = await fs.stat(path.join("target", "deploy", `${IDL.name}.so`));
    binarySize[IDL.name] = stat.size;
  });

  after(async () => {
    const bench = await BenchData.open();
    await bench.update({ binarySize });
  });
});
