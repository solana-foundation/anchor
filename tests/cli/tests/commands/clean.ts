import path from "path";
import { setupTest, runCommands, anchorCommand, diffTest } from "@/lib";

const COMMAND_NAME = "clean";

describe(COMMAND_NAME, () => {
  it("should succeed", () => {
    const { testDir } = setupTest(COMMAND_NAME);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);

    runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand("clean")],
    });

    diffTest(COMMAND_NAME);
  });
});
