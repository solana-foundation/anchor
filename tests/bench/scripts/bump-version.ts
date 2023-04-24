/**
 * Bump the version of all benchmark related files by changing the `Unreleased`
 * version to a new version and adding a new `Unreleased` version.
 */

import { BenchData } from "./utils";

(async () => {
  const newVersion = process.argv[2];

  if (!newVersion) {
    console.error("Usage: anchor run bump-version -- <VERSION>");
    process.exit(1);
  }

  // Bump bench data
  const bench = await BenchData.open();
  bench.bumpVersion(newVersion);
  await bench.save();

  // Bump markdown files
  await BenchData.forEachMarkdown((markdown) => {
    markdown.bumpVersion(newVersion);
  });
})();
