#!/usr/bin/env node

import { AbiCoder, solidityPacked } from "ethers";
import { Command } from "commander";

const program = new Command();

program
  .name("crypto-params-cli")
  .description("A CLI for specifying an input validator and BFV parameters")
  .version("1.0.0")
  .requiredOption("--input-validator <validator>", "input validation rule")
  .requiredOption("--bfv-params <params>", "BFV scheme parameters")
  .action((options) => {
    const abiCoder = new AbiCoder();

    const out = abiCoder.encode(
      ["bytes", "address"],
      [
        options.bfvParams,
        options.inputValidator,
      ],
    );

    console.log(out);
  });

program.parse(process.argv);
