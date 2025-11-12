// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { execa } from "execa";
import { readFile, writeFile, rm } from "fs/promises";
import { resolve } from "path";
import replaceInFile from "replace-in-file";

try {
  // Build WASM with web and node target - generates index.js and index_bg.wasm.
  const distWeb = resolve(process.cwd(), "dist/web");
  const distNode = resolve(process.cwd(), "dist/node");

  await execa("wasm-pack", [
    "build",
    "../../crates/zk-inputs-wasm",
    "--target=web",
    `--out-dir=${distWeb}`,
    "--no-pack",
    "--out-name=index",
  ]);
  await execa("wasm-pack", [
    "build",
    "../../crates/zk-inputs-wasm",
    "--target=nodejs",
    `--out-dir=${distNode}`,
    "--no-pack",
    "--out-name=index",
  ]);

  // Convert WASM binary to base64 for bundler compatibility.
  const wasmBinary = await readFile("./dist/web/index_bg.wasm");
  const base64Src = `export default '${wasmBinary.toString("base64")}';\n`;

  // Parallel cleanup and JS modification to prevent Next.js and other bundlers static analysis issues.
  await Promise.all([
    await Promise.all([
      rm("./dist/web/index_bg.wasm", { force: true }),
      rm("./dist/web/index_bg.wasm.d.ts", { force: true }),
      rm("./dist/web/.gitignore", { force: true }),
      rm("./dist/node/.gitignore", { force: true }),
    ]),
    replaceInFile({
      files: "./dist/web/index.js",
      from: /module_or_path\s*=\s*new URL\(['"]index_bg\.wasm['"],\s*import\.meta\.url\);\s*/g,
      to: "/* wasm URL disabled: load via @crisp-e3/zk-inputs/init */\n",
    }),
    writeFile("./dist/web/index_base64.js", base64Src),
  ]);
} catch (error) {
  console.error(error);
  process.exit(1);
}
