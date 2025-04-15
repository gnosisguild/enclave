export const markdown = `
   # Enclave: Encrypted Execution Environments (E3)
   ## White Paper
   Authors: Auryn Macmillan, Nathan Ginnever, Marvin Lin
   
   ### Abstract
   This white paper introduces Enclave, an open-source protocol for Encrypted Execution Environments (E3) enabling privacy-preserving applications. Enclave integrates Fully Homomorphic Encryption (FHE) with a unique combination of threshold cryptography, zero-knowledge proofs (ZKPs), and a committee of staked nodes to enable secure computations on encrypted data without compromising confidentiality. In addition to Enclave, the paper also details its flagship offering: an implementation of CRISP (Collusion-Resistant Impartial Selection Protocol), a Secret Ballot protocol aimed at preventing collusion and mitigating vulnerabilities in governance using Enclave. We conclude with notes on limitations and comparisons between CRISP and the Minimal Anti-Collusion Infrastructure (MACI).
   
   ### Introduction
   In a digital landscape fraught with vulnerabilities and privacy concerns, safeguarding sensitive information while maximising its utility has become a paramount concern for individuals and organisations globally. The inability of existing systems to sufficiently address these vulnerabilities highlights the need for robust data privacy and computational integrity measures, which would allow valuable insights to be derived from data without exposing private information. It is within this context that we introduce Enclave: an open-source protocol for Encrypted Execution Environments (E3) enabling organisations to create the next generation of privacy-preserving applications.
   
   Enclave represents an advancement in the field of privacy-preserving technologies, offering comprehensive infrastructure for organisations to simultaneously safeguard and leverage private data. By integrating multiparty FHE with a unique combination of threshold cryptography, zero-knowledge proofs (ZKPs), and token economics, Enclave enables secure computations on encrypted data without exposing the underlying information. It is designed to accommodate any use case where multiple users encrypt private data to preserve confidentiality, with strong guarantees that the inputs and intermediate states cannot be revealed.
   
   At the heart of Enclave's operational model lies its approach to data-processing enclaves, secured by a committee of staked nodes. These nodes, entrusted with overseeing data-processing operations, form a decentralized network that ensures the integrity of computations and the confidentiality of private data through a consensus mechanism driven by stakeholder participation. By utilising staking mechanisms, proportional fees, and punitive measures, this framework not only enhances transparency and accountability in the data-processing environment, but also provides strong economic and cryptographic guarantees around privacy, liveness, and computational integrity.
   
   In this white paper, we delve into the core principles and functionalities of Enclave, including a detailing of our flagship offering: CRISP (Collusion-Resistant Impartial Selection Protocol), a Secret Ballot protocol capable of preventing collusion and mitigating vulnerabilities in governance and decision-making systems. By exploring the intricacies of how our technology works with CRISP, we demonstrate how the flexibility of Enclave can be leveraged by any organisation for their specific use cases and needs, upholding the principles of privacy, integrity, and trust in their data-driven initiatives while maximising the value derived from their data assets.
   
   ## Contents
   - [Overview](#overview)
   - [Preamble](#preamble)
   - [Structure of Enclave](#structure-of-enclave)
     - [Actors](#actors)
     - [Phases](#phases)
   - [Cancellation](#cancellation)
   - [Staking](#staking)
   - [Fees](#fees)
   - [Penalties](#penalties)
     - [Inactive Node](#inactive-node)
     - [Failure To Report Outcome](#failure-to-report-outcome)
     - [Failure To Decrypt](#failure-to-decrypt)
     - [Non-Deterministic Computation](#non-deterministic-computation)
     - [Intermediate Decryption](#intermediate-decryption)
     - [Forced Decommissioning](#forced-decommissioning)
   - [Governance](#governance)
   - [Flagship Use Case: Secret Ballots](#flagship-use-case-secret-ballots)
     - [Preamble](#preamble-1)
     - [Structure of CRISP Secret Ballots](#structure-of-crisp-secret-ballots)
     - [Setup](#setup)
     - [Poll Creation](#poll-creation)
     - [Voter Registration](#voter-registration)
     - [Voting](#voting)
     - [Key Switching](#key-switching)
     - [Computing and Publishing Results](#computing-and-publishing-results)
     - [Triggering Onchain Actions](#triggering-onchain-actions)
     - [Mitigation of Common Attacks](#mitigation-of-common-attacks)
   - [Limitations](#limitations)
     - [Corrupt Registry](#corrupt-registry)
   - [Comparison to MACI](#comparison-to-maci)
   - [Contributors](#contributors)
   
   ### Overview
   #### Preamble
   Enclave is an open-source protocol for Encrypted Execution Environments (E3) that enables organisations to simultaneously safeguard and leverage private data in their applications. Through advanced cryptography, privacy-preserving mechanism design, and multiparty computation, Enclave provides strong economic and cryptographic guarantees around privacy, liveness, and computational integrity.
   
   These guarantees are due largely to the unique structure of Enclave, which utilises data-processing enclaves composed of five groups of actors overseeing the initiation, execution, and decryption of computations through a multiphase process. Additionally, by pairing cryptographic assurances with staking mechanisms, proportional fees, punitive measures, and built-in governance, Enclave is designed to be resilient and adaptable, offering a general solution to build privacy-preserving applications suitable for various sectors, use cases, and organisational needs.
   
   ### Structure of Enclave
   #### Actors
   There are five groups of actors in Enclave:
   - Token Holders: As the top-level governance body, Enclave token holders are responsible for setting protocol parameters, overseeing protocol upgrades, and facilitating dispute resolution.
   - Execution Modules: Enclave is a modular framework, allowing the choice of many different Execution Modules in which to run encrypted computations. Broadly, Execution Modules fall into two categories: (1) Provable (like RISC Zero’s virtual machine or Arbitrum’s WAVM) and (2) Oracle-based. The former provides cryptographic guarantees of correct execution, while the latter provides economic guarantees of correct execution.
   - Cypher Nodes: Cypher Nodes are responsible for creating threshold public keys and decrypting the cyphertext output for each requested computation. Cypher Nodes can be registered by anyone staking Enclave tokens.
   - Requesters: Anyone can request an E3 from the Enclave network by calling the corresponding smart contract entrypoint and depositing a bond proportional to the number, threshold, and duration of Cypher Nodes that they request.
   - Data Providers: Individuals and systems providing inputs to a requested E3. Data Providers contribute data encrypted to the public threshold key that is created by and published on chain by the Cypher Nodes selected for a requested E3.
   
   #### Phases
   Enclave leverages a multiphase process to facilitate the interaction between its various Actors, ultimately producing publicly verifiable outputs with strong economic guarantees around data privacy. Figure 1 shows a visual description of these interactions.
   
   ![CRISP Diagram](/#/crisp-diagram.webp)
   
   ### Figure 1
   1. **Request & Bond**
      Requesters can request an FHE computation from Enclave at any time. To request a computation, the following must be defined:
      - The FHE computation to be performed
      - The Execution Module for the computation, along with any additional parameters required
      - The quantity Cypher Nodes to be selected
      - The threshold for each type of node that must agree on the computed outputs
      - The timestamp for the Input Deadline, after which no new inputs will be accepted
      - The duration for which the nodes must be available
      Along with the computation request, Requesters must also provide a proportional bond to ensure a minimum reward for the requested nodes for performing their duties. Anyone can add to this bond at any point prior to Decryption, the final phase in the process.
   
   2. **Node Selection**
      When a computation is requested, the required number of Cypher Nodes are selected from the pool of available nodes via sortition. The selected Cypher Nodes immediately generate and publish shared public keys with the requested thresholds.
   
   3. **Input Window**
      Once the selected Cypher Nodes have published their shared public key, Data Providers can create and publish their encrypted inputs to the computation up until the Input Deadline.
      To publish an input, one must provide both the input and a corresponding zero-knowledge proof (ZKP) to ensure the input is valid for the requested computation. A ZKP is required to ensure the Data Provider knows the plaintext of the input, that the input is correctly formed, that the input cannot be used in any other context, and that the input passes any other validation logic required by the selected computation.
   
   4. **Metering**
      With inputs finalised, the fee for the requested computation can be calculated according to the Execution Module’s metering method. Computation will not proceed until the bond for the requested computation is equal to or greater than the published fee. See the Fees section for details on how fees are calculated.
   
   5. **Computation**
      Once the fee has been published, the selected Execution Module runs the computation and provides the cyphertext output.
   
   6. **Decryption**
      Once the selected Execution Module provides the cyphertext output of a requested computation, a threshold of the Cypher Nodes must collectively decrypt and publish the output of the computation. Once the output of the computation is revealed, Cypher Node duties are complete. Each node can then claim their proportional share of the bond and should dispose of the keys used for this committee, treating them as toxic waste.
   
   ### Cancellation
   If a requested computation cannot be completed (for example, if the output turns out to be non-deterministic), then the selected Execution Module can cancel the computation. In this case, a portion of the bond is used to purchase and burn CRISP tokens, and the remainder is paid to the selected Cypher Nodes.
   
   ### Staking
   Anyone can register a new Cypher Node by staking an amount of CRISP tokens. After registration, new nodes must wait for a registration delay period before they can be selected for Cypher duties. The registration delay period enables Requesters to reasonably predict the current Cypher Node set when requesting a computation.
   Nodes can request to be decommissioned at any time. Prior to being decommissioned, nodes must remain active for a decommission delay period, after which they will no longer be selected for duties. Nodes must also remain active to complete any duties for which they have been selected. Once its decommission delay has passed and all assigned duties have been completed, a node may be decommissioned and its staking deposit returned.
   
   ### Fees
   When requesting a computation from Enclave, Requesters must first deposit a bond proportional to the number and threshold of Cypher Nodes requested, the duration for which they are required to be available, and any additional costs required by the selected Execution Module. This deposit reserves the requested Cypher Nodes, providing an economic guarantee that they will be online to decrypt the output of the requested computation. However, the deposit does not pay for the computation itself. Different execution environments will have different fee structures.
   
   Cypher Nodes are subject to a penalty and forcefully decommissioned if they are proven to have provided their share of the decryption data from any input or intermediate states for a computation on which they were a committee member. To ensure there is a long-term disincentive for intermediate decryption, even for decommissioned Cypher Nodes, a portion of the Cypher Node rewards are paid out immediately after Decryption, with the remaining portion subject to a cliff and vesting schedule, along with slashing conditions.
   
   ### Penalties
   There are numerous slashing conditions for Cypher Nodes, with penalties ranging from loss of rewards for a given duty round to forced decommissioning. The cases and penalties are detailed below.
   
   #### Inactive Node
   Cypher Nodes that fail to provide their share of the data necessary for decrypting a requested computation forfeit their share of the decryption fee and are also subject to a small penalty used to cover the additional gas cost incurred due to their missing signature.
   
   #### Failure To Report Outcome
   If the selected Execution Module does not provide the cyphertext output of the requested computation in a timely manner, the computation request is cancelled. In this case, a small portion of the bond is paid out to the selected Cypher Node committee, while the remainder of the bond for the computation is returned to the requester.
   
   #### Failure To Decrypt
   If the selected Cypher Node committee does not provide the decrypted plaintext from the cyphertext output agreed on by the selected Execution Module in a timely manner, the computation request is cancelled. In this case, the bond for the computation is returned to the Requester. Any nodes that did not provide their share of the decryption data are subject to a penalty and are forcefully decommissioned. Penalties collected are split proportionally between the remaining members of the selected Cypher Node committee.
   
   #### Non-Deterministic Computation
   In order for nodes to reach consensus on the output, a requested computation must be deterministic. If the selected Execution Module reports that the requested computation has a non-deterministic output, the bond for that output is partially distributed between the selected nodes and partially used to purchase and burn CRISP tokens.
   
   #### Intermediate Decryption
   Maintaining the privacy of all encrypted inputs and intermediary states is a critical feature of Enclave. Attempts to decrypt anything encrypted to a Cypher Node committee, except the agreed upon output of a requested computation, is punishable by forced decommissioning, along with slashing a portion of the offending node’s stake. A portion of the slashed stake is burned, while the remaining portion is allocated to the account that notified the network of the slashable offence.
   
   #### Forced Decommissioning
   If a node’s effective stake is ever reduced to half of the minimum stake, it is immediately decommissioned and will no longer be selected for duties. However, nodes must also remain active to complete any duties for which they had already been selected. Once all assigned duties have been completed, a forcefully decommissioned node may claim the remainder of its staking deposit.
   
   ### Governance
   Enclave has several variables that may require periodic adjustments to maintain the protocol's fairness, performance, and responsiveness to evolving requirements. The setting of these variables is controlled by Enclave governance, which is responsible for protocol upgrades, dispute resolution, and the following parameter settings:
   - cancellation burn percentage
   - staking deposit amounts
   - registration delay period 
   - decommission delay period 
   - inactive node penalty amounts 
   - intermediate slashing penalty and burn ratio
   
   ### Flagship Use Case: Secret Ballots
   #### Preamble
   Collusion-Resistant Impartial Selection Protocol (CRISP) is a strategic response to the persisting challenges in contemporary decision-making systems. Collusion, data breaches, and compromised privacy continue to undermine governance and decision-making, necessitating the development of an advanced protocol capable of mitigating potential vulnerabilities while also preventing forms of collusion.
   
   To address these threats, CRISP reconfigures current decision-making paradigms using Enclave. Serving as a modern embodiment of the secret (Australian) ballot, the protocol leverages Enclave to align economic incentives with the goals of fairness, transparency, and integrity, cultivating robust and equitable decision-making environments.
   
   #### Structure of CRISP Secret Ballots
   This section details the components built atop Enclave’s general-purpose core in order to enable modern, collusion-resistant ballots. This includes smart contracts that mediate Requester and Data Provider interactions with Enclave, along with a vote-tallying computation to be run via Enclave.
   
   The CRISP implementation includes a Zodiac-compatible module that can be used to control any contract account that conforms to the Zodiac IAvatar interface; a Safe, for example.
   
   #### Setup
   To enable a compatible account for control by CRISP secret ballots, one must deploy a CRISP module and activate it on the account to be controlled. This process involves specifying the following:
   - The account to be controlled by the module.
   - The ID of the designated vote-tallying computation.
   - The designated execution environment, along with any additional parameters.
   - The required number and threshold of Cypher Nodes.
   - The duration for each poll.
   - The address of a voter registry contract.
   The specific steps for enabling a module on a smart contract account may differ between implementations and are not detailed in this document. Once the CRISP module is enabled on the account, it can be utilised to create polls that can ultimately trigger the account to make any arbitrary call.
   
   #### Poll Creation
   Anyone can propose a transaction to be executed by a smart contract account with a CRISP module enabled. To do so, one must call a function on the CRISP module and provide the following: the hash of the proposal description, the hash of the transaction payload, and the bond for the computation required by the CRISP network.
   
   This will register the proposal in the CRISP module and request the computation from the CRISP network, using the threshold and duration parameters defined in the CRISP module’s setup.
   
   #### Voter Registration
   In each poll, every voter must have a cryptographic keypair to cast their vote, which must be registered in a Voter Registry smart contract. This contract is responsible for:
   - Specifying the necessary proof for validating messages in the FHE computation.
   - Enforcing any voter eligibility criteria mandated by the poll.
   - Determining the vote weight for each registered voter.
   For example, a Voter Registry may require that voters hold a minimum amount of a token or be provably a member on a predefined list of eligible voters. Importantly, voting keys are distinct from the keypairs a voter might use to otherwise interact onchain; voting keys are single-use keys specific to the current poll, disposable once the poll concludes, and should not be reused for other polls or in other contexts.
   
   #### Voting
   Votes are submitted directly to the CRISP smart contract as cyphertext encrypted to the shared key provided by the selected Cypher Nodes, along with a zero-knowledge proof that the encrypted message represents a valid vote format was signed by a registered voter, and cannot be re-used in any other proposal. This setup allows any account to submit a vote message on behalf of any user.
   
   Voters have the option to change their vote at any time prior to the Input Deadline by submitting an encrypted message to the CRISP smart contract. Only the latest message will be counted in the tallied results.
   
   #### Key Switching
   Voters have the option to change their voting keys at any time prior to the Input Deadline by submitting a correctly formatted key change message to the CRISP smart contract, encrypted to the private key provided by the selected Cypher Nodes. Only messages signed by a voter’s most recent valid key will be counted in the tallied results.
   
   #### Computing and Publishing Results
   Once the Input Deadline has passed, the selected Execution Module will compute the tally cyphertext, and the selected Cypher Nodes will both decrypt the output and post the hash of plaintext results onchain.
   
   #### Triggering Onchain Actions
   Once the selected Cypher Nodes post the hash of the plaintext results onchain, anyone can call a function to execute the attached transaction payloads. This execution requires providing proof of the results supplied by the selected Cypher Nodes.
   
   #### Mitigation of Common Attacks
   Our voting system built on CRISP is designed to be resilient against a host of practical and theoretical attacks. This section details several of these attack vectors, along with the corresponding mitigations in our voting implementation.
   
   In each scenario, Alice and Bob are both registered voters. Alice and Bob are both outspoken supporters of the Banana Party. Chuck runs the web service through which voters submit their votes in polls and is also a supporter of the Durian Party. Other characters may also be introduced.
   
   ##### Censorship
   Knowing that Alice and Bob will each likely vote for the Banana Party, Chuck chooses to ignore Alice’s and Bob’s votes when they are submitted to his web service via the voting application, neglecting to post the votes on chain.
   
   As Chuck was unable to provide a valid transaction receipt to Alice and Bob showing that their votes had been submitted onchain, Alice and Bob can choose to directly post their votes to the CRISP contract without permission or censorship from any intermediaries, like Chuck. By submitting their votes directly, or via an alternate relaying service, Alice and Bob are able to circumvent Chuck’s attempt to censor their votes.
   
   Chuck could also attempt to mount a similar attack attempting to deny voter registration for any given poll, but the mitigation would be similar.
   
   ##### Receipt Sharing
   Knowing that Alice and Bob will each likely vote for the Banana Party, Chuck decides to offer Alice a bribe to vote for the Durian Party, rather than her original preference. Chuck offers to pay the bribe to Alice on the condition that Alice can prove how they voted.
   
   Alice’s optimal behaviour is to accept the bribe from Chuck, vote in the same way they would have without the bribe, and simply supply Chuck with a fake proof that is indistinguishable from a legitimate vote. To do this, Alice could cast a vote for the Durian party, then submit a keychange message to change their voting key, invalidating their previous vote, and then finally cast a vote for the Banana party with the new key. Alice can share the receipt of the first vote with Chuck. However, Chuck has no way of guaranteeing whether the key used to cast the vote was Alice’s valid voting key. So Chuck must simply take Alice’s word for it.
   
   ##### Proxy Voting
   Knowing that Alice and Bob will each likely vote for the Banana Party, Chuck decides to offer Alice a bribe to vote for the Durian Party, rather than her original preference. Chuck offers to pay the bribe to Alice on the condition that Chuck is granted permission to cast Alice’s vote. To comply, Alice must submit a key change message switching her voting key to one supplied by Chuck.
   
   As with the case of receipt sharing, Alice’s optimal behaviour is to accept Chuck’s bribe and to share the receipt of a corresponding key change message with Chuck, after having already switched voting keys to another key unknown to Chuck. Unfortunately for Chuck, there is no way to ensure that Alice has not previously registered or switched to a different voting key, and no way to determine if the message to change keys to Chuck’s voting key is in fact invalid.
   
   ##### Forced Abstention
   Knowing that Alice and Bob will each likely vote for the Banana Party, Chuck decides to force both Alice and Bob to abstain from voting (either by bribery or more coercive forms of collusion). To comply with Chuck’s demand, Alice and Bob must not be caught registering for or casting a vote in the poll.
   
   Alice and Bob’s optimal behaviour is to vote as normal, while taking care to not leave any identifiable traces. When registering and submitting votes onchain, Alice and Bob can submit messages through a relayer other than the one controlled by Chuck or from a fresh address which cannot be linked to their identity. Registration involves providing a proof that the voter is on the registry, but does not require the voter to identify themselves in plaintext or submit the registration message from a specific account. Similarly, when casting a vote, nothing identifiable is published in plaintext, and no cyphertext aside from the result will ever be decrypted by the Cypher Nodes.
   
   ### Limitations
   #### Corrupt Registry
   Like any other voting implementation, CRISP is dependent on a functioning voter registry. If the voter registration process is compromised, allowing an attacker to either deny registration or take control of a voter’s account before they select their voter keys, then the attacker can successfully corrupt the system.
   
   The latter type of attack could be mitigated in a variety of ways, depending on the scope and trust assumptions appropriate for a given poll. However, a general rule of thumb is to ensure there is enough other value associated with each voter's account that they would be unwilling to share the credentials with a third party. At the very least, this makes such attacks more costly and less scalable.
   
   ### Comparison to MACI
   CRISP’s design is heavily inspired by MACI, a protocol originally proposed by Vitalik Buterin for collusion-resistant voting leveraging zero-knowledge proofs (ZKPs). CRISP differs from MACI primarily by employing fully homomorphic encryption (FHE) and threshold cryptography. This approach allows CRISP to establish an arbitrarily large network of nodes for trust distribution, contrasting with MACI's reliance on a single trusted coordinator.
   
   While MACI relies on an honest coordinator assumption for privacy, meaning the coordinator has unrestricted access to all of the inputs and intermediate states and is trusted to not divulge them, CRISP provides strong economic guarantees around privacy, as Cypher Nodes are subject to slashing if anyone can prove that they’ve attempted to decrypt any input or intermediate cyphertext in a computation.
   
   ### Contributors
   A special thank you to the following people for their early contributions to, and reviews of, the CRISP white paper.
   
   - Alex Espinosa
   - Anthony Leutenegger
   - Disruption Joe
   - Koh Wei Jie
   - Mike Chan
   - Vitalik Buterin
   - Yuet Loo Wong
   `
