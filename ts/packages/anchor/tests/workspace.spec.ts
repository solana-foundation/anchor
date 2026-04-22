import * as fs from "fs";
import * as path from "path";
import * as os from "os";
import camelcase from "camelcase";

// Replicates the IDL-lookup logic from workspace.ts so it can be unit-tested
// in isolation without spinning up a full Anchor workspace.
function resolveIdlFileName(idlDirPath: string, programName: string): string {
  let dirEntries: string[];
  try {
    dirEntries = fs.readdirSync(idlDirPath);
  } catch (err: any) {
    if (err.code === "ENOENT") {
      throw new Error(
        `IDL directory not found at \`${idlDirPath}\`. Did you run \`anchor build\`?`
      );
    }
    throw err;
  }

  const jsonFiles = dirEntries.filter(
    (name) =>
      path.extname(name) === ".json" &&
      fs.statSync(path.join(idlDirPath, name)).isFile()
  );

  const fileName = jsonFiles.find(
    (name) => camelcase(path.parse(name).name) === programName
  );

  if (!fileName) {
    if (jsonFiles.length === 0) {
      throw new Error(
        `No IDL files found in \`${idlDirPath}\`. Did you run \`anchor build\`?`
      );
    }
    const available = jsonFiles
      .map((n) => path.parse(n).name)
      .sort()
      .join(", ");
    throw new Error(
      `Failed to find IDL for program \`${programName}\`.\n` +
        `Available programs in \`${idlDirPath}\`: ${available}\n` +
        `Ensure the following all use the same snake_case name:\n` +
        `  - \`[lib].name\` in Cargo.toml\n` +
        `  - \`#[program]\` module name in your Rust program\n` +
        `  - Program key in Anchor.toml under \`[programs.<cluster>]\``
    );
  }

  return fileName;
}

describe("workspace IDL resolution", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "anchor-idl-test-"));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("finds the correct IDL when the program name matches exactly", () => {
    fs.writeFileSync(path.join(tmpDir, "user_g_market.json"), "{}");
    expect(resolveIdlFileName(tmpDir, "userGMarket")).toBe(
      "user_g_market.json"
    );
  });

  it("throws a helpful error listing available IDLs when name has a typo", () => {
    // Replicates: Cargo.toml has `name = "user_g_market"` (singular)
    // but workspace references `user_g_markets` (plural)
    fs.writeFileSync(path.join(tmpDir, "user_g_market.json"), "{}");

    expect(() => resolveIdlFileName(tmpDir, "userGMarkets")).toThrowError(
      /Failed to find IDL for program `userGMarkets`/
    );
    expect(() => resolveIdlFileName(tmpDir, "userGMarkets")).toThrowError(
      /user_g_market/
    );
    expect(() => resolveIdlFileName(tmpDir, "userGMarkets")).toThrowError(
      /\[lib\]\.name.*Cargo\.toml/
    );
    expect(() => resolveIdlFileName(tmpDir, "userGMarkets")).toThrowError(
      /\#\[program\].*Rust program/
    );
    expect(() => resolveIdlFileName(tmpDir, "userGMarkets")).toThrowError(
      /Anchor\.toml/
    );
  });

  it("lists multiple available IDLs sorted alphabetically on typo", () => {
    fs.writeFileSync(path.join(tmpDir, "my_program.json"), "{}");
    fs.writeFileSync(path.join(tmpDir, "another_program.json"), "{}");

    expect(() => resolveIdlFileName(tmpDir, "missingProgram")).toThrowError(
      /another_program, my_program/
    );
  });

  it("throws a friendly error when the idl/ directory doesn't exist", () => {
    const nonExistentDir = path.join(tmpDir, "idl");
    expect(() => resolveIdlFileName(nonExistentDir, "anyProgram")).toThrowError(
      /IDL directory not found.*Did you run `anchor build`/
    );
  });

  it("throws a friendly error when the idl/ directory exists but is empty", () => {
    expect(() => resolveIdlFileName(tmpDir, "anyProgram")).toThrowError(
      /No IDL files found.*Did you run `anchor build`/
    );
  });

  it("ignores non-JSON files and directories when scanning for IDLs", () => {
    fs.writeFileSync(path.join(tmpDir, "user_g_market.json"), "{}");
    fs.writeFileSync(path.join(tmpDir, "readme.txt"), "ignore me");
    fs.mkdirSync(path.join(tmpDir, "user_g_market"));

    // Should still find the correct JSON IDL and not trip over the txt or dir
    expect(resolveIdlFileName(tmpDir, "userGMarket")).toBe(
      "user_g_market.json"
    );
  });

  it("does not list non-JSON files in the available programs error", () => {
    fs.writeFileSync(path.join(tmpDir, "user_g_market.json"), "{}");
    fs.writeFileSync(path.join(tmpDir, "readme.txt"), "ignore me");

    expect(() => resolveIdlFileName(tmpDir, "missingProgram")).toThrowError(
      /user_g_market/
    );
    expect(() => resolveIdlFileName(tmpDir, "missingProgram")).not.toThrowError(
      /readme/
    );
  });
});
