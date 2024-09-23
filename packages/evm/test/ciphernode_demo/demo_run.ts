import {
  loadFixture,
  mine,
  time,
} from "@nomicfoundation/hardhat-network-helpers";
import { LeanIMT } from "@zk-kit/lean-imt";
import { expect } from "chai";
import { ZeroHash } from "ethers";
import { ethers } from "hardhat";
import { poseidon2 } from "poseidon-lite";

import { deployEnclaveFixture } from "../fixtures/Enclave.fixture";
import { deployCiphernodeRegistryFixture } from "../fixtures/MockCiphernodeRegistry.fixture";
import { deployComputeProviderFixture } from "../fixtures/MockComputeProvider.fixture";
import { deployDecryptionVerifierFixture } from "../fixtures/MockDecryptionVerifier.fixture";
import { deployE3ProgramFixture } from "../fixtures/MockE3Program.fixture";
import { deployInputValidatorFixture } from "../fixtures/MockInputValidator.fixture";
import { PoseidonT3Fixture } from "../fixtures/PoseidonT3.fixture";
import { naiveRegistryFilterFixture } from "../fixtures/NaiveRegistryFilter.fixture";

import { Enclave__factory } from "../../types/factories/contracts/Enclave__factory";

const abiCoder = ethers.AbiCoder.defaultAbiCoder();
const AddressTwo = "0x0000000000000000000000000000000000000002";
const AddressSix = "0x0000000000000000000000000000000000000006";
const encryptionSchemeId =
  "0x0000000000000000000000000000000000000000000000000000000000000001";
const newEncryptionSchemeId =
  "0x0000000000000000000000000000000000000000000000000000000000000002";

const FilterFail = AddressTwo;
const FilterOkay = AddressSix;

const data = "0xda7a";
const dataHash = ethers.keccak256(data);
const proof = "0x1337";

// Hash function used to compute the tree nodes.
const hash = (a: bigint, b: bigint) => poseidon2([a, b]);

describe("Enclave", function () {
  async function setup() {
    const [owner, node_1, node_2, node_3] = await ethers.getSigners();

    //const EnclaveContract = await ethers.getContractFactory("Enclave");
    let poseidonT3 = "0x610178da211fef7d417bc0e6fed39f05609ad788";
    const EnclaveContract = await (
      await ethers.getContractFactory("Enclave", {
        libraries: {
          PoseidonT3: poseidonT3,
        },
      })
    );
    const enclave = EnclaveContract.attach(
      "0x0b306bf915c4d645ff596e518faf3f9669b97016" // The deployed contract address
    );

    const RegistryContract = await (
      await ethers.getContractFactory("CiphernodeRegistryOwnable", {
        libraries: {
          PoseidonT3: poseidonT3,
        },
      })
    );
    const registry = RegistryContract.attach(
      "0x959922be3caee4b8cd9a407cc3ac1c251c2007b1" // The deployed contract address
    );

    const filter = await naiveRegistryFilterFixture(
      owner.address,
      await registry.getAddress(),
    );

    return {
      owner,
      node_1,
      node_2,
      node_3,
      enclave,
      registry,
      filter,
    };
  }

  describe("run e3 round()", function () {
    it("run system", async function () {
      const { owner, node_1, node_2, node_3, enclave, registry, filter } = await loadFixture(setup);

      await registry.addCiphernode(await owner.getAddress());
      await registry.addCiphernode(await node_1.getAddress());
      await registry.addCiphernode(await node_2.getAddress());
      await registry.addCiphernode(await node_3.getAddress());
      await new Promise(r => setTimeout(r, 2000));

      await registry.requestCommittee(
        Math.floor(Math.random() * 1000),
        await filter.getAddress(),
        [2, 2] as [number, number],
      );
      expect(await enclave.owner()).to.equal(owner.address);
    });

    // it("correctly sets ciphernodeRegistry address", async function () {
    //   const { mocks, enclave } = await loadFixture(setup);
    //   expect(await enclave.ciphernodeRegistry()).to.equal(
    //     await mocks.registry.getAddress(),
    //   );
    // });

    // it("correctly sets max duration", async function () {
    //   const { enclave } = await loadFixture(setup);
    //   expect(await enclave.maxDuration()).to.equal(60 * 60 * 24 * 30);
    // });
  });
});