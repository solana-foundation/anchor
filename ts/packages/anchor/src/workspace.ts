import * as toml from "toml";
import { snakeCase } from "snake-case";
import { Program } from "./program/index.js";
import { isBrowser } from "./utils/common.js";
import { Idl } from "./idl.js";

/**
 * The `workspace` namespace provides a convenience API to automatically
 * search for and deserialize [[Program]] objects defined by compiled IDLs
 * in an Anchor workspace.
 *
 * This API is for Node only.
 */
const workspace = new Proxy(
  {},
  {
    get(workspaceCache: { [key: string]: Program }, programName: string) {
      if (isBrowser) {
        throw new Error("Workspaces aren't available in the browser");
      }

      // Converting `programName` to snake_case enables the ability to use any
      // of the following to access the workspace program:
      // `workspace.myProgram`, `workspace.MyProgram`, `workspace["my-program"]`...
      programName = snakeCase(programName);

      // Check whether the program name contains any digits
      if (/\d/.test(programName)) {
        // Numbers cannot be properly converted from camelCase to snake_case,
        // e.g. if the `programName` is `myProgram2`, the actual program name could
        // be `my_program2` or `my_program_2`. This implementation assumes the
        // latter as the default and always converts to `_numbers`.
        //
        // A solution to the conversion of program names with numbers in them
        // would be to always convert the `programName` to camelCase instead of
        // snake_case. The problem with this approach is that it would require
        // converting everything else e.g. program names in Anchor.toml and IDL
        // file names which are both snake_case.
        programName = programName
          .replace(/\d+/g, (match) => "_" + match)
          .replace("__", "_");
      }

      // Return early if the program is in cache
      if (workspaceCache[programName]) return workspaceCache[programName];

      const fs = require("fs");
      const path = require("path");

      // Override the workspace programs if the user put them in the config.
      const anchorToml = toml.parse(fs.readFileSync("Anchor.toml"));
      const clusterId = anchorToml.provider.cluster;
      const programEntry = anchorToml.programs?.[clusterId]?.[programName];

      let idlPath: string;
      let programId;
      if (typeof programEntry === "object" && programEntry.idl) {
        idlPath = programEntry.idl;
        programId = programEntry.address;
      } else {
        idlPath = path.join("target", "idl", `${programName}.json`);
      }

      if (!fs.existsSync(idlPath)) {
        throw new Error(
          `${idlPath} doesn't exist. Did you run \`anchor build\`?`
        );
      }

      const idl: Idl = JSON.parse(fs.readFileSync(idlPath));
      if (programId) {
        idl.address = programId;
      }
      workspaceCache[programName] = new Program(idl);

      return workspaceCache[programName];
    },
  }
);

export default workspace;
