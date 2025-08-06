import { EnclaveSDK, FheProtocol } from "@gnosis-guild/enclave-sdk"
import { Enclave, Enclave__factory as EnclaveFactory } from "@gnosis-guild/enclave/types"
import { expect } from "chai"
import fs from "fs"
import path from "path"
import { Signer, AbiCoder } from "ethers"
import { SemaphoreEthers } from '@semaphore-protocol/data';
import { Group, generateNoirProof, SemaphoreNoirProof, initSemaphoreNoirBackend, Identity } from '@semaphore-protocol/core';
import dotenv from "dotenv"
import { hexToBytes, encodeAbiParameters, parseAbiParameters, bytesToHex } from 'viem';
import { CRISPProgram, CRISPProgram__factory as CRISPProgramFactory } from "../types"

dotenv.config()

const rpcUrl = process.env.RPC_URL!

interface Round {
    id: number 
}

interface State {
    id: number 
    chain_id: number 
    enclave_address: string 
    status: string 
    vote_count: number 
    start_time: number 
    duration: number 
    expiration: number 
    start_block: number 
    committee_public_key: Uint8Array
    emojis: string[2]
}

const merkleTreeDepth = 10
const semaphoreIdentity = new Identity()

const abi = parseAbiParameters(
    '(uint256,uint256,uint256,uint256,uint256,bytes)'
);

export function encodeSemaphoreProof(
    { merkleTreeDepth, merkleTreeRoot, nullifier, message, scope, proofBytes }: SemaphoreNoirProof
): Uint8Array {
    const hex = encodeAbiParameters(abi, [
        [
            BigInt(merkleTreeDepth),
            BigInt(merkleTreeRoot),
            BigInt(nullifier),
            BigInt(message),
            BigInt(scope),
            bytesToHex(proofBytes),
        ]
    ]);

    return hexToBytes(hex);
}

export const generateSemaphoreProof = async (
    startBlock: number,
    groupId: bigint,
    semaphoreAddress: string
) => {
    let ethersRef = new SemaphoreEthers(rpcUrl, {
        address: semaphoreAddress,
        startBlock,
    });

    const groupMembers = await ethersRef.getGroupMembers(groupId.toString());
    const group = new Group(groupMembers);
    const scope = groupId.toString();
    const message = "0";
    const noirBackend = await initSemaphoreNoirBackend(merkleTreeDepth);
    const fullProof: SemaphoreNoirProof = await generateNoirProof(semaphoreIdentity, group, message, scope, noirBackend, true);
    const proofBytes = encodeSemaphoreProof(fullProof);

    return proofBytes
}

/**
 * @note In order to run these tests, you need the relayer server up and running.
 */
describe("CRISP contracts", () => {
    const enclaveAddress = "0xe7f1725E7734CE288F8367e1Bb143E90bb3F0512" 
    const crispProgramAddress = "0xc6e7DF5E7b4f2A278906862b61205850344D4e7d"

    const sdk = EnclaveSDK.create({
        rpcUrl: "http://localhost:8545",
        contracts: {
            enclave: enclaveAddress,
            ciphernodeRegistry: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
        },
        chainId: 31337,
        protocol: FheProtocol.BFV,
        privateKey: "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
    })
    const server = "http://localhost:4000"
    const stateEndpoint = `${server}/rounds/current`

    let signer: Signer
    let enclaveContract: Enclave 
    let crispProgramContract: CRISPProgram

    const circuit = JSON.parse(fs.readFileSync(path.resolve(__dirname, "crisp_circuit.json"), "utf-8")) 

    before(async () => {
        const { ethers } = await import("hardhat");

        [signer] = await ethers.getSigners();

        enclaveContract = EnclaveFactory.connect(enclaveAddress, signer)

        crispProgramContract = CRISPProgramFactory.connect(crispProgramAddress, signer)
    })

    describe("voting", async () => {
        it("should get the current round", async () => {
            const data = await fetch(stateEndpoint)
            const json = await data.json() as Round 

            const roundId = json.id 

            expect(roundId).to.be.not.null
            expect(roundId).to.be.gte(0)

            const state = await fetch(`${server}/state/lite`, {
                method: "POST",
                body: JSON.stringify({
                    round_id: roundId
                }),
                headers: {
                    "Content-Type": "application/json"
                }
            })
            const stateJson = await state.json() as State 

            expect(stateJson).to.be.instanceOf(Object)
        })

        it("should allow to vote", async () => {
            const data = await fetch(stateEndpoint)
            const json = await data.json() as Round 

            const roundId = json.id 

            const state = await fetch(`${server}/state/lite`, {
                method: "POST",
                body: JSON.stringify({
                    round_id: roundId
                }),
                headers: {
                    "Content-Type": "application/json"
                }
            })
            const stateJson = await state.json() as State 

            expect(stateJson).to.be.instanceOf(Object)

            const publicKey = stateJson.committee_public_key 

            const semaphoreAddr = await crispProgramContract.semaphore()
            const groupId = await crispProgramContract.groupIds(roundId)

            // register the member
            await crispProgramContract.registerMember(roundId, semaphoreIdentity.commitment)

            const { proof, encryptedVote } = await sdk.encryptNumberAndGenProof(0n, publicKey, circuit)
            const semaphoreProof = await generateSemaphoreProof(stateJson.start_block, groupId, semaphoreAddr)

            const encodedInputs = AbiCoder.defaultAbiCoder().encode(
                ["bytes", "bytes", "bytes32[]", "bytes"],
                [
                    semaphoreProof,
                    proof.proof,
                    proof.publicInputs,
                    encryptedVote
                ]
            )

            await enclaveContract.publishInput(roundId, encodedInputs)
        })
    })
})
