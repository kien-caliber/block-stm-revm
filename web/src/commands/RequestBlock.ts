import * as winston from "winston";
import * as expressAsyncHandler from "express-async-handler";
import { z } from "zod";
import * as path from "path";
import { runPevmOptimismChecker } from "../lib/process";

export function getHandler$RequestBlock({ outputDir }: { outputDir: string }) {
  return expressAsyncHandler(async (req, res) => {
    const params = {
      blocks: z.string().parse(req.body["blocks"]),
      rpcUrl: z.string().nullish().parse(req.body["rpc-url"]),
    };
    const blockNumbers = params.blocks
      .split(",")
      .map((word) => z.coerce.number().safe().parse(word.trim()));

    const logPath = path.join(outputDir, `request.${Date.now()}.log`);
    const logger = winston.createLogger({
      level: "info",
      format: winston.format.json(),
      transports: [
        new winston.transports.File({ filename: logPath }),
        new winston.transports.Console(),
      ],
    });

    logger.info({ blockNumbers, rpcUrl: params.rpcUrl });
    res.redirect("/" + logPath);

    for (const blk of blockNumbers) {
      logger.info({ blk });
      try {
        const result = await runPevmOptimismChecker({
          blk,
          rpcUrl: params.rpcUrl,
        });
        logger.info({ result });
      } catch (error) {
        logger.error({ error });
      }
    }
  });
}
