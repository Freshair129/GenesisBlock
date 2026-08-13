import { spawnSync } from "node:child_process";

const script = "scripts/validate_doc_status.py";
const args = [script, "docs"];
const candidates = process.platform === "win32"
  ? [["py", ["-3", ...args]], ["python", args]]
  : [["python3", args], ["python", args]];

for (const [command, commandArgs] of candidates) {
  const result = spawnSync(command, commandArgs, { stdio: "inherit" });
  if (result.error?.code === "ENOENT") continue;
  process.exit(result.status ?? 1);
}

console.error("Unable to find a Python interpreter (tried py/python3/python).");
process.exit(1);
