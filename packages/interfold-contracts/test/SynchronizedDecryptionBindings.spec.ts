// SPDX-License-Identifier: LGPL-3.0-only
import { expect } from "chai";
import { network } from "hardhat";

const { ethers } = await network.connect();
const P =
  21888242871839275222246405745257275088548364400416034343698204186575808495617n;
const field = (v: bigint | number) => ethers.toBeHex(v, 32);
const encode = (inputs: string[]) =>
  ethers.AbiCoder.defaultAbiCoder().encode(
    ["bytes", "bytes32[]"],
    ["0x01", inputs],
  );

describe("Synchronized decryption DKG bindings", function () {
  async function fixture() {
    const circuit = await (
      await ethers.getContractFactory("MockCircuitVerifier")
    ).deploy();
    await circuit.setReturnValue(true);
    const registry = await (
      await ethers.getContractFactory("MockCiphernodeRegistry")
    ).deploy();
    const verifier = await (
      await ethers.getContractFactory("BfvDecryptionVerifier")
    ).deploy(
      await circuit.getAddress(),
      await registry.getAddress(),
      field(11),
      field(12),
      1,
    );
    const ids = [0, 2]; // Selected rows are not a prefix of the full N=3 committee.
    const sk = [field(21), field(22)];
    const keys = Array.from({ length: 6 }, (_, i) =>
      field((1n << 128n) + BigInt(100 + i)),
    );
    await registry.setDkgAnchors(7, ids, sk, []);
    await registry.setDkgPrfKeyCommitments(7, keys);
    const inputs = Array(76).fill(field(0)) as string[];
    inputs[0] = field(11);
    inputs[1] = field(12);
    inputs[6] = field(13);
    inputs[8] = field(1);
    inputs[9] = field(3);
    inputs[10] = sk[0];
    inputs[11] = sk[1];
    for (let owner = 0; owner < 2; ++owner) {
      for (let peer = 0; peer < 3; ++peer) {
        inputs[64 + owner * 3 + peer] = keys[owner * 3 + peer];
        const peerRow = ids.indexOf(peer);
        if (peerRow >= 0)
          inputs[70 + owner * 3 + peer] = keys[peerRow * 3 + ids[owner]];
      }
    }
    const plaintext = ethers.keccak256(new Uint8Array(400));
    const verify = (values: string[]) =>
      verifier.verify.staticCall(
        7,
        ethers.ZeroHash,
        plaintext,
        ethers.ZeroHash,
        field(13),
        encode(values),
      );
    return { registry, verifier, inputs, verify, keys };
  }

  it("accepts non-prefix recipients and canonical key commitments wider than u64", async function () {
    const { inputs, verify } = await fixture();
    expect(await verify(inputs)).to.equal(true);
  });

  for (const index of [64, 66, 67, 69, 70, 72, 73, 75]) {
    it(`rejects a substituted directed key at public field ${index}`, async function () {
      const { verifier, inputs, verify } = await fixture();
      inputs[index] = field(999);
      await expect(verify(inputs)).to.be.revertedWithCustomError(
        verifier,
        "DkgAnchorMismatch",
      );
    });
  }

  it("does not accept legacy ESM-shaped proofs as PRF proofs", async function () {
    const { verifier, inputs, verify } = await fixture();
    await expect(verify(inputs.slice(0, 64))).to.be.revertedWithCustomError(
      verifier,
      "InvalidPublicInputsLength",
    );
  });

  it("rejects a noncanonical field even in an unused peer slot", async function () {
    const { verifier, inputs, verify } = await fixture();
    inputs[65] = field(P);
    await expect(verify(inputs))
      .to.be.revertedWithCustomError(verifier, "NonCanonicalPublicInput")
      .withArgs(65);
  });

  it("rejects missing PRF anchors rather than falling back to ESM anchors", async function () {
    const { registry, verifier, inputs, verify } = await fixture();
    await registry.setDkgPrfKeyCommitments(7, []);
    await expect(verify(inputs)).to.be.revertedWithCustomError(
      verifier,
      "DkgAnchorMismatch",
    );
  });

  it("binds the aggregate SK independently of the PRF keys", async function () {
    const { verifier, inputs, verify } = await fixture();
    inputs[10] = field(99);
    await expect(verify(inputs)).to.be.revertedWithCustomError(
      verifier,
      "DkgAnchorMismatch",
    );
  });

  it("does not treat appended keys as plaintext coefficients", async function () {
    const { verifier, inputs, verify } = await fixture();
    inputs[14] = field(1n << 64n);
    await expect(verify(inputs))
      .to.be.revertedWithCustomError(verifier, "MessageCoefficientOutOfRange")
      .withArgs(14);
  });
});

describe("Synchronized fold attestation v2", function () {
  async function signedFixture(version = "2") {
    const registry = await (
      await ethers.getContractFactory("MockCiphernodeRegistry")
    ).deploy();
    const verifier = await (
      await ethers.getContractFactory("DkgFoldAttestationVerifier")
    ).deploy();
    const operators = (await ethers.getSigners())
      .slice(0, 3)
      .sort((a, b) =>
        a.address.toLowerCase().localeCompare(b.address.toLowerCase()),
      );
    await registry.setCommitteeNodes(
      7,
      operators.map((operator) => operator.address),
    );
    const keys = Array.from({ length: 6 }, (_, i) => field(100 + i));
    const inputs = Array(16).fill(field(0)) as string[];
    inputs[3] = field(2);
    inputs[7] = field(21);
    inputs[8] = field(22);
    keys.forEach((key, i) => {
      inputs[10 + i] = key;
    });
    const { chainId } = await ethers.provider.getNetwork();
    const domain = {
      name: "InterfoldDkgFoldAttestation",
      version,
      chainId,
      verifyingContract: await verifier.getAddress(),
    };
    const types = {
      DkgFoldAttestation: [
        { name: "registry", type: "address" },
        { name: "e3Id", type: "uint256" },
        { name: "partyId", type: "uint256" },
        { name: "skAggCommit", type: "bytes32" },
        { name: "prfKeyCommitsHash", type: "bytes32" },
      ],
    };
    const attestations = [];
    const bindings = [];
    for (const [row, partyId] of [0, 2].entries()) {
      const payload = {
        registry: await registry.getAddress(),
        e3Id: 7,
        partyId,
        skAggCommit: inputs[7 + row],
        prfKeyCommitsHash: ethers.keccak256(
          ethers.concat(keys.slice(row * 3, row * 3 + 3)),
        ),
      };
      attestations.push([
        partyId,
        payload.skAggCommit,
        payload.prfKeyCommitsHash,
        await operators[partyId].signTypedData(domain, types, payload),
      ]);
      bindings.push([partyId, operators[partyId].address]);
    }
    const bundle = ethers.AbiCoder.defaultAbiCoder().encode(
      [
        "tuple(uint256 partyId,bytes32 skAggCommit,bytes32 prfKeyCommitsHash,bytes signature)[]",
        "tuple(uint256 partyId,address node)[]",
      ],
      [attestations, bindings],
    );
    const registryAddress = await registry.getAddress();
    const verify = () =>
      verifier.verify.staticCall(
        registryAddress,
        chainId,
        7,
        encode(inputs),
        bundle,
      );
    return { verifier, verify, inputs, keys };
  }

  it("authenticates non-prefix operators and returns the complete directed grid", async function () {
    const { verify, keys } = await signedFixture();
    const [ids, sk, returnedKeys] = await verify();
    expect([...ids]).to.deep.equal([0n, 2n]);
    expect([...sk]).to.deep.equal([field(21), field(22)]);
    expect([...returnedKeys]).to.deep.equal(keys);
  });

  it("rejects a substituted key row even with the same aggregate SK", async function () {
    const { verifier, verify, inputs } = await signedFixture();
    inputs[12] = field(999);
    await expect(verify()).to.be.revertedWithCustomError(
      verifier,
      "InvalidFoldAttestation",
    );
  });

  it("rejects signatures from the pre-migration domain version", async function () {
    const { verifier, verify } = await signedFixture("1");
    await expect(verify()).to.be.revertedWithCustomError(
      verifier,
      "InvalidFoldAttestation",
    );
  });
});
