import fs from "fs";
import path from "path";
import { execFileSync } from "child_process";

export const SCRIPT_DIR = path.resolve(__dirname, "..", "..");

export const WORKSPACE_DIR = path.resolve(SCRIPT_DIR, "..", "..");
export const EXPECTED_DIR = path.join(SCRIPT_DIR, "expected");
export const INITIALIZE_DIR = path.join(SCRIPT_DIR, "initialize");
export const OUTPUT_DIR = path.join(SCRIPT_DIR, "output");
export const MOCK_BIN_DIR = path.resolve(__dirname, "..", "mock-bin");

export function setupTest(testPath: string) {
  const testDir = path.join(OUTPUT_DIR, testPath);
  if (fs.existsSync(testDir)) fs.rmSync(testDir, { recursive: true });

  // Only copy from initialize if the directory exists
  const initDir = path.join(INITIALIZE_DIR, testPath);
  if (fs.existsSync(initDir))
    fs.cpSync(initDir, testDir, {
      recursive: true,
    });
  else
    fs.mkdirSync(testDir, {
      recursive: true,
    });

  return {
    testDir,
  };
}

export function diffTest(testPath: string) {
  const expectedDir = path.join(EXPECTED_DIR, testPath);
  const outputDir = path.join(OUTPUT_DIR, testPath);

  // try {
  runCommands({
    commands: [`diff -u -r "${expectedDir}" "${outputDir}"`],
  });
  // }
  // catch(e){
  //   console.error(getErrorMessage(e));
  // }
}

export interface RunCommandsArgs {
  env?: Record<string, string>;
  cwd?: string;
  commands: string[];
  strictMode?: boolean;
  overrideEnv?: boolean;
  prependPath?: string[];
}

export function runCommands({
  env,
  cwd,
  commands,
  strictMode = true,
  overrideEnv,
  prependPath,
}: RunCommandsArgs) {
  commands = [...commands];
  if (strictMode) commands.unshift("set -euo pipefail");

  const resolvedEnv = {
    ...(overrideEnv ? {} : process.env),
    ...env,
  };
  if (prependPath?.length) {
    const PATH = [...prependPath];
    if (resolvedEnv.PATH) PATH.push(resolvedEnv.PATH);
    resolvedEnv.PATH = PATH.join(path.delimiter);
  }

  const script = commands.join("\n");
  try {
    execFileSync("bash", ["-c", script], {
      cwd,
      env: resolvedEnv,
      maxBuffer: 1024 * 1024 * 25,
      windowsHide: true,
    });
  } catch (e: any) {
    const errorMessage = getErrorMessage(e);
    const stdout = e?.stdout ?? "";
    const stderr = e?.stderr ?? "";

    let message = [
      `error code ${e?.code}: ${errorMessage}`,
      "",
      "while executing script:",
      script,
      "",
      "stdout:",
      stdout,
      "",
      "stderr:",
      stderr,
    ].join("\n");

    throw new Error(message);
  }
}

export function getErrorMessage(e: any): string {
  return e?.message ?? e?.toString?.() ?? JSON.stringify(e, null, 2);
}

export function anchorCommand(command: string): string {
  const override = process.env.MOCK_ANCHOR_BIN;
  const binary = override || `${WORKSPACE_DIR}/target/debug/anchor`;
  return `${binary} ${command}`;
}

export interface PatchWorkspaceArgs {
  workspaceDir: string;
}

export function patchWorkspace({ workspaceDir }: PatchWorkspaceArgs) {
  fs.rmSync(path.join(workspaceDir, "app"), {
    recursive: true,
  });
  fs.rmSync(path.join(workspaceDir, "target"), {
    recursive: true,
  });
}

export interface PatchProgramIdArgs {
  workspaceDir: string;
  programName: string;
  newProgramId?: string;
}

export function patchProgramId({
  workspaceDir,
  programName,
  newProgramId = "aaLWzFHRPNhQwft1971qmPg2Q5eHwsHEWivqSkCDo9x",
}: PatchProgramIdArgs) {
  const programRustName = programName.replaceAll("-", "_");

  // fix declare_id!()
  const libRs = path.join(
    workspaceDir,
    "programs",
    programName,
    "src",
    "lib.rs",
  );
  replaceInFile({
    file: libRs,
    find: /declare_id!.*/,
    replace: `declare_id!("${newProgramId}");`,
  });

  const anchorToml = path.join(workspaceDir, "Anchor.toml");
  replaceInFile({
    file: anchorToml,
    find: new RegExp(`(${programName.replaceAll("-", ".")}) = .*`),
    replace: (match: string) => match.split(" = ")[0] + ` = "${newProgramId}"`,
  });

  // delete keypair, if exists
  const keypairFile = path.join(
    workspaceDir,
    "target",
    "deploy",
    `${programRustName}-keypair.json`,
  );
  fs.rmSync(keypairFile, {
    force: true,
  });
}

export function replaceInFile({
  file,
  find,
  replace,
}: {
  file: string;
  find: RegExp | string;
  replace: string | ((substring: string, ...args: any[]) => string);
}): void {
  let contents = fs.readFileSync(file).toString("utf8");
  contents = contents.replace(find, replace as string);
  fs.writeFileSync(file, contents);
}
