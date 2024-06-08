import * as expressAsyncHandler from "express-async-handler";
import { extractFromOutputDir } from "../lib/summary";

export function getHandler$GetBlocks({ outputDir }: { outputDir: string }) {
  return expressAsyncHandler(async (_req, res) => {
    const blocks = await extractFromOutputDir(outputDir);
    res.render("blocks", { blocks });
  });
}
