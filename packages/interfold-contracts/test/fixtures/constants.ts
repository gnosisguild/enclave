// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
// Shared test constants. Importable by every spec file.
import { ethers } from "./connection";

// ── Addresses ────────────────────────────────────────────────────────────────
export const ADDRESS_ONE = "0x0000000000000000000000000000000000000001";
export const ADDRESS_TWO = "0x0000000000000000000000000000000000000002";

// ── Time ─────────────────────────────────────────────────────────────────────
export const ONE_HOUR = 60 * 60;
export const ONE_DAY = 24 * ONE_HOUR;
export const THREE_DAYS = 3 * ONE_DAY;
export const SEVEN_DAYS = 7 * ONE_DAY;
export const THIRTY_DAYS = 30 * ONE_DAY;

// ── Sortition ────────────────────────────────────────────────────────────────
export const SORTITION_SUBMISSION_WINDOW = 10;

// ── Encryption scheme ───────────────────────────────────────────────────────
// Derived from the same string the BFV verifier wrappers and
// `MockE3Program` use (`keccak256("fhe.rs:BFV")`) so the test constant
// stays aligned with the contracts if either side ever changes.
export const ENCRYPTION_SCHEME_ID = ethers.id("fhe.rs:BFV");

// ── Fake ciphertext / proof payloads used across spec files ──────────────────
export const DATA = "0xda7a";
export const PROOF = "0x1337";

// ── BFV parameter sets (abi.encode(uint256 degree, uint256 modulus, uint256[] moduli)) ──
const abiCoder = ethers.AbiCoder.defaultAbiCoder();

/** Small BFV params (degree 512). Used by `Interfold.spec` & `Pricing.spec`. */
export const BFV_PARAMS_DEFAULT = abiCoder.encode(
  ["uint256", "uint256", "uint256[]"],
  [
    ethers.toBigInt(512),
    ethers.toBigInt(10),
    [ethers.toBigInt("0xffffee001"), ethers.toBigInt("0xffffc4001")],
  ],
);

/** Production-sized BFV params (degree 2048). Used by `E3Integration.spec`. */
export const BFV_PARAMS_LARGE = abiCoder.encode(
  ["uint256", "uint256", "uint256[]"],
  [
    ethers.toBigInt(2048),
    ethers.toBigInt(1032193),
    [ethers.toBigInt("18014398492704769")],
  ],
);

// ── Timeout configs ──────────────────────────────────────────────────────────
/** 1h / 1h / 1h — used by short-lifecycle tests. */
export const DEFAULT_TIMEOUT_CONFIG = {
  dkgWindow: ONE_HOUR,
  computeWindow: ONE_HOUR,
  decryptionWindow: ONE_HOUR,
};

/** 1d / 3d / 1d — used by long-lifecycle integration tests. */
export const LARGE_TIMEOUT_CONFIG = {
  dkgWindow: ONE_DAY,
  computeWindow: THREE_DAYS,
  decryptionWindow: ONE_DAY,
};

// ── Bonding defaults (passed to BondingRegistry constructor) ─────────────────
/** 10 USDC ticket price (6-decimal stable). */
export const TICKET_PRICE = ethers.parseUnits("10", 6);
/** 1000 license tokens (18-decimal) per active operator. */
export const LICENSE_REQUIRED_BOND = ethers.parseEther("1000");
/** Minimum ticket balance (in ticket units, not USDC). */
export const MIN_TICKET_BALANCE = 5;
