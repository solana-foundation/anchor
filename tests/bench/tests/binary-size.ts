import * as fs from "fs/promises";
import path from "path";

import { BenchData, BinarySize } from "../scripts/utils";

const IDL = require("../target/idl/bench.json");

describe("Binary size", () => {
  const binarySize: BinarySize = {};

  it("Measure binary size", async () => {
    const stat = await fs.stat(
      path.join("target", "deploy", `${IDL.metadata.name}.so`)
    );
    binarySize[IDL.metadata.name] = stat.size;
  });

  after(async () => {
    const bench = await BenchData.open();
    await bench.update({ binarySize });
  });
});
