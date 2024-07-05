import * as fs from "fs";
import * as ChildProcess from "child_process";

export function runPevmOptimismChecker({
  blk,
  rpcUrl,
}: {
  blk: number;
  rpcUrl: string | null | undefined;
}): Promise<void> {
  return new Promise((resolve, reject) => {
    const statusPath = `output/${blk}.status.txt`;
    fs.writeFileSync(statusPath, "PENDING", "utf-8");

    const args = [
      "cargo",
      "run",
      "--example",
      "check-optimism",
      "--release",
      "--",
      `${blk}`,
    ];
    rpcUrl && args.push("--rpc-url", rpcUrl);
    const process = ChildProcess.spawn("/usr/bin/env", args, { stdio: "pipe" });
    process.stdin.end();
    const stdoutLogStream = fs.createWriteStream(`output/${blk}.stdout.log`);
    process.stdout.pipe(stdoutLogStream);
    const stderrLogStream = fs.createWriteStream(`output/${blk}.stderr.log`);
    process.stderr.pipe(stderrLogStream);

    process.on("exit", (code, signal) => {
      if (!code && !signal) {
        fs.writeFileSync(statusPath, "OK", "utf-8");
        resolve();
      } else {
        fs.writeFileSync(statusPath, `ERROR (${code || signal})`, "utf-8");
        reject({ code, signal });
      }
    });
    process.on("error", (err) => {
      fs.writeFileSync(statusPath, "ERROR", "utf-8");
      reject(err);
    });
  });
}
