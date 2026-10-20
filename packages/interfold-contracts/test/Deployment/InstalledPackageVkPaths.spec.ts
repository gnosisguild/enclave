// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
import { expect } from "chai";
import fs from "fs";
import { mock } from "node:test";
import path from "path";

import {
  SECURE_16384_MINIMUM_BFV_CONFIG,
  getBfvDecryptionSubCircuitVkHashPaths,
  getBfvPkSubCircuitVkHashPaths,
  getBfvPkVkBindingHashPaths,
  getBfvV2SubCircuitVkHashPaths,
  getBfvV2VkBindingHashPaths,
} from "../../scripts/utils";

describe("installed-package BFV VK paths", function () {
  const previous = process.env.INTERFOLD_CIRCUITS_DIR;
  const release = path.resolve("release-circuits");

  beforeEach(function () {
    // Model an installed package: no ancestor has the monorepo package marker.
    mock.method(fs, "existsSync", () => false);
    process.env.INTERFOLD_CIRCUITS_DIR = release;
  });

  afterEach(function () {
    mock.restoreAll();
    if (previous === undefined) delete process.env.INTERFOLD_CIRCUITS_DIR;
    else process.env.INTERFOLD_CIRCUITS_DIR = previous;
  });

  it("uses the active insecure pair without requiring a checkout", function () {
    expect(getBfvPkSubCircuitVkHashPaths().nodesFold).to.equal(
      path.join(
        release,
        "insecure/minimum/default/recursive_aggregation/nodes_fold/nodes_fold.vk_tree_hash",
      ),
    );
  });

  it("resolves every independent l-BFV manifest under the selected release pair", function () {
    const config = SECURE_16384_MINIMUM_BFV_CONFIG;
    const prefix = path.join(release, "secure-16384/minimum") + path.sep;
    const legacy = getBfvPkVkBindingHashPaths(config);
    const v2 = getBfvV2VkBindingHashPaths(config);
    expect(legacy).to.have.length(15);
    expect(v2).to.have.length(11);
    for (const file of [
      ...legacy,
      ...v2,
      ...Object.values(getBfvV2SubCircuitVkHashPaths(config)),
    ]) {
      expect(file.startsWith(prefix)).to.equal(true);
    }
    expect(getBfvDecryptionSubCircuitVkHashPaths(config).c7).to.equal(
      path.join(
        prefix,
        "default/threshold/decrypted_shares_aggregation_wide/decrypted_shares_aggregation_wide.vk_hash",
      ),
    );
  });

  it("fails closed when no release circuit directory is configured", function () {
    delete process.env.INTERFOLD_CIRCUITS_DIR;
    expect(() =>
      getBfvV2VkBindingHashPaths(SECURE_16384_MINIMUM_BFV_CONFIG),
    ).to.throw("set INTERFOLD_CIRCUITS_DIR");
  });
});
