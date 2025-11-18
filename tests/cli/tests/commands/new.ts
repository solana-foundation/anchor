import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  patchProgramId,
  patchWorkspace,
  diffTest,
} from "@/lib";

const COMMAND_NAME = "new";

describe(COMMAND_NAME, () => {
  it("should succeed", () => {
    const { testDir } = setupTest(COMMAND_NAME);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const programName = "another-program";

    runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand(`new ${programName}`)],
    });

    patchWorkspace({
      workspaceDir,
    });
    patchProgramId({
      workspaceDir,
      programName,
      newProgramId: "bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
    });

    diffTest(COMMAND_NAME);
  });
});
