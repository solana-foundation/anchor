import fs from "fs";
import path from "path";
import { expect } from "chai";
import { setupTest, runCommands, anchorCommand } from "@/lib";

describe("keys", () => {
  it("keys list", () => {
    const commandName = "keys/list";

    const { testDir } = setupTest(commandName);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);

    const commandOutput = runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand("keys list")],
    });

    expect(commandOutput).to.eq(
      [
        "test_program: aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
        "another_program: bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
        "",
      ].join("\n"),
    );
  });

  it("keys sync", () => {
    const commandName = "keys/sync";

    const { testDir } = setupTest(commandName);
    const workspaceName = "test-program";
    const workspaceDir = path.join(testDir, workspaceName);

    const commandOutput = runCommands({
      cwd: workspaceDir,
      commands: [anchorCommand("keys sync")],
    });

    expect(commandOutput).to.contain(
      "Updated to aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
    );
    expect(commandOutput).to.contain(
      "Updated to bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ",
    );

    const anchorTomlFile = path.join(workspaceDir, "Anchor.toml");
    const anchorToml = fs.readFileSync(anchorTomlFile, { encoding: "utf-8" });
    expect(anchorToml).to.contain(
      [
        "[programs.localnet]",
        'another-program = "bbHgTM8c4goW91FVeYMUUE8bQgGaqNZLNRLaoK4HqnJ"',
        'test_program = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x"',
      ].join("\n"),
      `keys sync did not patch file correctly: ${anchorTomlFile}`,
    );

    const testProgramLibRsFile = path.join(
      workspaceDir,
      "test-program",
      "src",
      "lib.rs",
    );
    const testProgramLibRs = fs.readFileSync(testProgramLibRsFile, {
      encoding: "utf-8",
    });
    expect(testProgramLibRs).to.contain(
      'declare_id!("aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");',
      `keys sync did not patch file correctly: ${testProgramLibRsFile}`,
    );

    const anotherProgramLibRsFile = path.join(
      workspaceDir,
      "another-program",
      "src",
      "lib.rs",
    );
    const anotherProgramLibRs = fs.readFileSync(anotherProgramLibRsFile, {
      encoding: "utf-8",
    });
    expect(anotherProgramLibRs).to.contain(
      'declare_id!("aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x");',
      `keys sync did not patch file correctly: ${anotherProgramLibRsFile}`,
    );
  });
});
