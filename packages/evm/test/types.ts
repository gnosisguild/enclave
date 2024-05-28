import type { SignerWithAddress } from "@nomicfoundation/hardhat-ethers/dist/src/signer-with-address";

import type { Enclave } from "../types/Enclave";

type Fixture<T> = () => Promise<T>;

declare module "mocha" {
  export interface Context {
    enclave: Enclave;
    loadFixture: <T>(fixture: Fixture<T>) => Promise<T>;
    signers: Signers;
  }
}

export interface Signers {
  admin: SignerWithAddress;
}
