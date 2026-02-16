import fs from "fs";
import path from "path";
import { execFileSync } from "child_process";
import { getCaller } from "@/lib/stack";

export const SCRIPT_DIR = path.resolve(__dirname, "..", "..");

export const WORKSPACE_DIR = path.resolve(SCRIPT_DIR, "..", "..");
export const MOCK_BIN_DIR = path.resolve(__dirname, "..", "mock-bin");
export const TEMPLATE_DIR = path.resolve(SCRIPT_DIR, "templates");

export interface SetupTestArgs {
  templateName?: string;
  testSubpath?: string;
  basePath?: string;
  initDir?: string;
  testDir?: string;
}

// `testSubpath` defaults to "."
// `basePath` defaults to `${calling file's directory}/${testSubpath}`
// `initDir` defaults to `${basePath}/initialize`
// `testDir` defaults to `${basePath}/output`
//  if `testDir`, it's deleted
//  if `templateName` is specified, `${TEMPLATE_DIR}/${templateName}` is copied to `testDir`
//  else if `initDir` exists, it's copied to `testDir`
//  else, a new directory is created
export function setupTest({
  templateName,
  testSubpath,
  basePath,
  initDir,
  testDir,
}: SetupTestArgs = {}) {
  if (!basePath) {
    const caller = path.dirname(getCaller());
    basePath = caller;
    if (testSubpath) basePath = path.join(caller, testSubpath);
  }

  // template name is specified, overried init dir
  if (templateName) {
    const templatePath = path.join(TEMPLATE_DIR, templateName);
    if (!fs.existsSync(templatePath))
      throw new Error(
        `could not find template ${templateName} in ${TEMPLATE_DIR}`,
      );
    initDir = templatePath;
  }

  initDir ??= path.join(basePath, "initialize");
  testDir ??= path.join(basePath, "output");

  if (fs.existsSync(testDir)) rmDir(testDir);

  // Only copy from initialize if the directory exists
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

export function rmDir(path: string) {
  if (fs.existsSync(path))
    fs.rmSync(path, {
      recursive: true,
    });
}

export interface DiffTestArgs {
  testSubpath?: string;
  basePath?: string;
  testDir?: string;
  expectedDir?: string;
}

// `basePath` defaults to uses the calling file's directory
// `testDir` defaults to `${basePath}/output`
// `expectedDir` defaults to `${basePath}/expected`
// runs diff between `testDir` and `expectedDir`
export function diffTest({
  testSubpath,
  basePath,
  testDir,
  expectedDir,
}: DiffTestArgs = {}) {
  if (!basePath) {
    const caller = path.dirname(getCaller());
    basePath = caller;
    if (testSubpath) basePath = path.join(caller, testSubpath);
  }

  expectedDir ??= path.join(basePath, "output");
  testDir ??= path.join(basePath, "expected");

  // try {
  runCommands({
    commands: [`diff -u -r "${expectedDir}" "${testDir}"`],
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
    return execFileSync("bash", ["-c", script], {
      cwd,
      env: resolvedEnv,
      maxBuffer: 1024 * 1024 * 25,
      windowsHide: true,
      encoding: "utf-8",
    });
  } catch (e: any) {
    const errorMessage = getErrorMessage(e);
    const stdout = e?.stdout ?? "";
    const stderr = e?.stderr ?? "";

    let message = [
      `error code ${e?.code ?? "<none>"}: ${errorMessage}`,
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
  rmDir(path.join(workspaceDir, "app"));
  rmDir(path.join(workspaceDir, "target"));
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
  let contents = fs.readFileSync(file, "utf8");
  contents = contents.replace(find, replace as string);
  fs.writeFileSync(file, contents);
}
