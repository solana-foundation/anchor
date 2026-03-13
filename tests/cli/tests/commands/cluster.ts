import path from "path";
import { setupTest, runCommands, anchorCommand, diffTest } from "@/lib";

const COMMAND_NAME = "cluster";

describe(COMMAND_NAME, () => {
  describe("list", () => {
    it("should succeed", () => {
      const { testDir } = setupTest(`${COMMAND_NAME}/list`);
      const outputPath = path.join(testDir, "output.txt");

      runCommands({
        cwd: testDir,
        commands: [anchorCommand(`cluster list > "${outputPath}"`)],
      });

      diffTest(`${COMMAND_NAME}/list`);
    });
  });
});

