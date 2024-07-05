import * as fs from "fs";
import * as path from "path";

const BLOCK_OUTPUT_FILE_TYPES = ["status", "stdout", "stderr"] as const;

export type BlockOutputFileType = (typeof BLOCK_OUTPUT_FILE_TYPES)[number];

export type BlockOutputFiles = Partial<Record<BlockOutputFileType, string>>;

export type BlockOutput = {
  blockNumber: number;
  files: BlockOutputFiles;
  status: string | undefined;
  updatedAt?: number;
};

export async function extractFromOutputDir(
  outputDir: string
): Promise<BlockOutput[]> {
  const results: BlockOutput[] = [];
  const names = await fs.promises.readdir(outputDir);

  for (const name of names) {
    const matches = /^([0-9]+)\.(.*)$/.exec(name);
    if (!matches) continue;
    const blockNumber = parseInt(matches[1]);
    let blk = results.find((item) => item.blockNumber == blockNumber);
    if (!blk)
      results.push((blk = { blockNumber, files: {}, status: undefined }));
    const fileType = BLOCK_OUTPUT_FILE_TYPES.find(
      (item) =>
        item + ".log" == matches[2] ||
        item + ".txt" == matches[2] ||
        item == matches[2]
    );
    if (fileType) blk.files[fileType] = path.join(outputDir, name);
  }

  for (const blk of results) {
    if (blk.files.stdout) {
      const mtime = await fs.promises
        .stat(blk.files.stdout)
        .then((stat) => stat.mtime.valueOf())
        .catch(() => undefined);
      if (mtime) {
        blk.updatedAt = Math.max(blk.updatedAt || 0, mtime);
      }
    }

    if (blk.files.stderr) {
      const mtime = await fs.promises
        .stat(blk.files.stderr)
        .then((stat) => stat.mtime.valueOf())
        .catch(() => undefined);
      if (mtime) {
        blk.updatedAt = Math.max(blk.updatedAt || 0, mtime);
      }
    }

    if (blk.files.status) {
      const text = await fs.promises
        .readFile(blk.files.status, "utf-8")
        .catch(() => undefined);
      blk.status = text;
    }
  }
  results.sort((a, b) => (b.blockNumber || 0) - (a.blockNumber || 0));
  return results;
}
