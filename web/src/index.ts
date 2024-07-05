import * as express from "express";
import * as ServeStatic from "serve-static";
import * as ServeIndex from "serve-index";
import * as fs from "fs";
import { getHandler$RequestBlock } from "./commands/RequestBlock";
import { getHandler$GetBlocks } from "./commands/GetBlocks";
// import { getHandler$RequestBlock } from "./commands/RequestBlock";
// import { getHandler$GetBlocks } from "./commands/GetBlocks";

if (!fs.existsSync("package.json")) {
  console.error("incorrect working directory");
  process.exit(1);
}

const HOST = process.env.HOST || "0.0.0.0";
const PORT = parseInt(process.env.PORT || "3036");

const app = express();

// https://expressjs.com/en/4x/api.html#app.set
app.set("json spaces", 2);
app.set("views", "views");
app.set("view engine", "ejs");

// https://expressjs.com/en/resources/middleware.html
app.use("/output", ServeIndex("output", { icons: true, view: "details" }));
app.use(
  "/output",
  ServeStatic("output", {
    setHeaders: (res, _path) => {
      res.setHeader("Content-Disposition", "inline");
      res.setHeader("Content-Type", "text/plain");
    },
  })
);
app.use("/package.json", ServeStatic("package.json"));
app.use(express.json());
app.use(express.urlencoded({ extended: true }));

app.get("/", (_req, res) => {
  res.render("home");
});

app.use("/blocks", getHandler$GetBlocks({ outputDir: "output" }));
app.post("/request", getHandler$RequestBlock({ outputDir: "output" }));

app.listen(PORT, HOST, () => {
  console.log(`Listening on ${HOST}:${PORT}`);
});
