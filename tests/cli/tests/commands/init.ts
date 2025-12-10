import path from "path";
import {
  setupTest,
  runCommands,
  anchorCommand,
  patchProgramId,
  patchWorkspace,
  diffTest,
} from "@/lib";

const COMMAND_NAME = "init";

describe(COMMAND_NAME, () => {
  it("should succeed", () => {
    const { testDir } = setupTest(COMMAND_NAME);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);
    const programName = workspaceName;

    runCommands({
      cwd: testDir,
      commands: [anchorCommand(`init ${workspaceName} --no-install --no-git`)],
    });

    patchWorkspace({
      workspaceDir,
    });
    patchProgramId({
      workspaceDir,
      programName,
    });

    diffTest(COMMAND_NAME);
  });
});
