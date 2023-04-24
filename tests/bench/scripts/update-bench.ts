/** Update Markdown files in /bench */

import { BenchData, Markdown } from "./utils";

(async () => {
  const bench = await BenchData.open();

  await BenchData.forEachMarkdown((markdown, fileName) => {
    if (fileName === "COMPUTE_UNITS.md") {
      const versions = bench.getVersions();

      // On the first version, compare with itself to update it with no changes
      versions.unshift(versions[0]);

      for (const i in versions) {
        const currentVersion = versions[i];
        const nextVersion = versions[+i + 1];

        if (currentVersion === "unreleased") {
          return;
        }

        const newComputeUnitsResult = bench.get(nextVersion).computeUnits;
        const oldComputeUnitsResult = bench.get(currentVersion).computeUnits;

        // Create table
        const table = Markdown.createTable(
          "Instruction",
          "Compute Units",
          "+/-"
        );

        bench.compareComputeUnits(
          newComputeUnitsResult,
          oldComputeUnitsResult,
          (ixName, newComputeUnits, oldComputeUnits) => {
            const percentChange = (
              (newComputeUnits / oldComputeUnits - 1) *
              100
            ).toFixed(2);

            let changeText;
            if (isNaN(oldComputeUnits)) {
              changeText = "N/A";
            } else if (+percentChange > 0) {
              changeText = `🔴 **+${percentChange}%**`;
            } else {
              changeText = `🟢 **${percentChange}%**`;
            }

            table.insert(ixName, newComputeUnits.toString(), changeText);
          },
          (ixName, computeUnits) => {
            table.insert(
              ixName,
              computeUnits.toString(),
              +i === 0 ? "N/A" : "-"
            );
          }
        );

        // Update version's table
        markdown.updateTable(nextVersion, table);
      }
    }
  });
})();
