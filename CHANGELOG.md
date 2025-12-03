## [0.1.6](https://github.com/gnosisguild/enclave/compare/v0.1.2...v0.1.6) (2025-12-03)

### Bug Fixes

- add some stability improvements to crisp_e2e test
  ([#936](https://github.com/gnosisguild/enclave/issues/936))
  ([8e14a29](https://github.com/gnosisguild/enclave/commit/8e14a2916fce36c62accdeecdbe690834d3431ea))
- change bound types in Greco to Field ([#972](https://github.com/gnosisguild/enclave/issues/972))
  ([4fd945e](https://github.com/gnosisguild/enclave/commit/4fd945e432a86b7d93745e6f490d61bd3a9da5b7))
- coderabbit config ([#893](https://github.com/gnosisguild/enclave/issues/893))
  ([82c09ea](https://github.com/gnosisguild/enclave/commit/82c09eae57f1635ac053d51a24ef99edfad126a9))
- contracts exports ([#732](https://github.com/gnosisguild/enclave/issues/732))
  ([c0686c6](https://github.com/gnosisguild/enclave/commit/c0686c6b42b351c07adf400c47d8cc5b2573f8e6))
- correctly parse custom params event ([#891](https://github.com/gnosisguild/enclave/issues/891))
  ([afc2b35](https://github.com/gnosisguild/enclave/commit/afc2b3503558a3d4cfdf7c15c519a6c3a18e7382))
- crisp circuit validation of encoded vote
  ([#973](https://github.com/gnosisguild/enclave/issues/973))
  ([d14826c](https://github.com/gnosisguild/enclave/commit/d14826c8be20c72165c8f48898e6539cdfe89afe))
- deploy risc0verifier with hardhat ([#894](https://github.com/gnosisguild/enclave/issues/894))
  ([e52d6e5](https://github.com/gnosisguild/enclave/commit/e52d6e59e9a1ff86bf2154afb67b89b73aaad4cb))
- deploy the right input validator ([#889](https://github.com/gnosisguild/enclave/issues/889))
  ([09c34ea](https://github.com/gnosisguild/enclave/commit/09c34ea2e295e9741e91fe5b9c4e6d139704b361))
- ensure we can deploy the full risc0 verifier
  ([#886](https://github.com/gnosisguild/enclave/issues/886))
  ([b94d0ef](https://github.com/gnosisguild/enclave/commit/b94d0ef81fb7cca19f7e5c622cdcd05fc52a2f7a))
- ensure we don't have uncommited files ([#676](https://github.com/gnosisguild/enclave/issues/676))
  ([a46e707](https://github.com/gnosisguild/enclave/commit/a46e70795655b8ff3a9896651f09f5ccee2592c7))
- ensure we push to the correct branch ([#770](https://github.com/gnosisguild/enclave/issues/770))
  ([9d630a6](https://github.com/gnosisguild/enclave/commit/9d630a6eab7c2329eb4603e1bebe48a82b35adcc))
- fix reentrancy issue in enclave contracts
  ([#752](https://github.com/gnosisguild/enclave/issues/752))
  ([3806a87](https://github.com/gnosisguild/enclave/commit/3806a870b39fa47a1b4b77f9484c0a1d74bfbaa4))
- increase timeout for crisp e2e for committee finalization
  ([#970](https://github.com/gnosisguild/enclave/issues/970))
  ([18a729d](https://github.com/gnosisguild/enclave/commit/18a729d750121d3e43e5bfb8ff4b035d422c21de))
- make submit ticket more gas efficient ([#965](https://github.com/gnosisguild/enclave/issues/965))
  ([558e13b](https://github.com/gnosisguild/enclave/commit/558e13bc52a0882a8f907593177dc9c318e23e19))
- pnpm install not working ([#921](https://github.com/gnosisguild/enclave/issues/921))
  ([f3f0631](https://github.com/gnosisguild/enclave/commit/f3f0631063c2f5c2a8364e84342209e4964846fa))
- refactor arcbytes to accept &[u8] ([#961](https://github.com/gnosisguild/enclave/issues/961))
  ([4ac6743](https://github.com/gnosisguild/enclave/commit/4ac6743ad2a87d314cd51abcfba668956ded7b72))
- release rust crates error ([#689](https://github.com/gnosisguild/enclave/issues/689))
  ([3c25929](https://github.com/gnosisguild/enclave/commit/3c25929f2317003c81d3a21d6b4fc9b1e44573cc))
- remove already published files from gitignore
  ([#680](https://github.com/gnosisguild/enclave/issues/680))
  ([283205d](https://github.com/gnosisguild/enclave/commit/283205dffc665d83cc741c07f697c1ecaf2d1d84))
- remove ci artifacts deep clean ([#681](https://github.com/gnosisguild/enclave/issues/681))
  ([242aac9](https://github.com/gnosisguild/enclave/commit/242aac96b9800043b0d24b5716b3262baefd4472))
- remove mac intel build and allow crates publishing to fail
  ([#777](https://github.com/gnosisguild/enclave/issues/777))
  ([8025c27](https://github.com/gnosisguild/enclave/commit/8025c277d5c4aa1005ab93d84d34158266458800))
- risc0 dev mode env var ([#810](https://github.com/gnosisguild/enclave/issues/810))
  ([7e09bad](https://github.com/gnosisguild/enclave/commit/7e09bad3373ecf4b7868840e2587d05a8a05160f))
- rust crate release error ([#694](https://github.com/gnosisguild/enclave/issues/694))
  ([56e9b12](https://github.com/gnosisguild/enclave/commit/56e9b12c2b319d1ea1081df4577b6b0cd0ccfc7d))
- rust crates release workflow ([#715](https://github.com/gnosisguild/enclave/issues/715))
  ([fc330c6](https://github.com/gnosisguild/enclave/commit/fc330c625742bce01def98ef3ccec5ae15fbdb96))
- rust releases ([#774](https://github.com/gnosisguild/enclave/issues/774))
  ([16870a4](https://github.com/gnosisguild/enclave/commit/16870a42973fccae7a376ccbc4b952f9e971fffa))
- small changes to aid crisp e2e test to run locally
  ([#926](https://github.com/gnosisguild/enclave/issues/926))
  ([92ff34e](https://github.com/gnosisguild/enclave/commit/92ff34eaeff1167bc9d125f14f5bda46f1389716))
- support crate contract path ([#1038](https://github.com/gnosisguild/enclave/issues/1038))
  ([c57aa99](https://github.com/gnosisguild/enclave/commit/c57aa9969b1b0295b8c72088d07422cae06dbc26))
- template init was not using up-to-date dependencies
  ([#1008](https://github.com/gnosisguild/enclave/issues/1008))
  ([b9c052f](https://github.com/gnosisguild/enclave/commit/b9c052fe90141be618c4d21af91787499994a329))
- update relative paths to use git ([#708](https://github.com/gnosisguild/enclave/issues/708))
  ([e0bd2bc](https://github.com/gnosisguild/enclave/commit/e0bd2bc7a5e2515013188fc7e40927630d1f6d58))
- use simulated network for hardhat ([#1024](https://github.com/gnosisguild/enclave/issues/1024))
  ([a6b5fd5](https://github.com/gnosisguild/enclave/commit/a6b5fd54ce209692ebda9e7ff641dde9231b0b0e))
- wait until start window before activating an e3
  ([#902](https://github.com/gnosisguild/enclave/issues/902))
  ([f797b3d](https://github.com/gnosisguild/enclave/commit/f797b3d2fd3742a11f92f917a8d2c578fea4bdd4))
- wasm init ([#740](https://github.com/gnosisguild/enclave/issues/740))
  ([58f7905](https://github.com/gnosisguild/enclave/commit/58f7905dd5bd33070be84b0bd5d88b5f44d98267))

### Features

- add a function to get an e3 public key ([#760](https://github.com/gnosisguild/enclave/issues/760))
  ([4db5dac](https://github.com/gnosisguild/enclave/commit/4db5dacf2f60872cfbafa16728b3da4f9244c248))
- add ciphertext addition to circuit ([#912](https://github.com/gnosisguild/enclave/issues/912))
  ([70857d4](https://github.com/gnosisguild/enclave/commit/70857d46df7e36a76e9961cf3221613522b1b45a))
- add contract verification on CRISP ([#885](https://github.com/gnosisguild/enclave/issues/885))
  ([f82c24a](https://github.com/gnosisguild/enclave/commit/f82c24a770871abe16711a45b6389c9a57d4b398))
- add crate for zk input generation [skip-line-limit]
  ([#901](https://github.com/gnosisguild/enclave/issues/901))
  ([c316a53](https://github.com/gnosisguild/enclave/commit/c316a53c03e6a5cb24400cea97261334cde8bf27))
- add dappnode pkg & update ci docker images tags [skip-line-limit]
  ([#1061](https://github.com/gnosisguild/enclave/issues/1061))
  ([46a40e7](https://github.com/gnosisguild/enclave/commit/46a40e77a591eb0b7a64a07f96e90aa7d0416f86))
- add dht get_record and set_record commands
  ([#904](https://github.com/gnosisguild/enclave/issues/904))
  ([09c4e2d](https://github.com/gnosisguild/enclave/commit/09c4e2d49cb32daf1aa7c51c9f76273f27172bb3))
- add ecdsa proving circuit ([#781](https://github.com/gnosisguild/enclave/issues/781))
  ([3acf773](https://github.com/gnosisguild/enclave/commit/3acf773b8664938b3d4a67291bb3cc3ee7f159b5))
- add functionality to encrypt a u64 vector
  ([#853](https://github.com/gnosisguild/enclave/issues/853))
  ([e9a8b9b](https://github.com/gnosisguild/enclave/commit/e9a8b9b42766a6c1500d6e8c92100326e4da8e1a))
- add historical events ordering on ciphernode startup
  ([#1012](https://github.com/gnosisguild/enclave/issues/1012))
  ([9287de5](https://github.com/gnosisguild/enclave/commit/9287de584bbd3afd98e4430ede6adb1d6b704505))
- add hybrid logical clock to codebase ([#1057](https://github.com/gnosisguild/enclave/issues/1057))
  ([c7d3a0f](https://github.com/gnosisguild/enclave/commit/c7d3a0f20c2c3d112db6949a135669831b2d13f7))
- add merkle tree proof inputs to circuit and sdk
  ([#917](https://github.com/gnosisguild/enclave/issues/917))
  ([ebd06c3](https://github.com/gnosisguild/enclave/commit/ebd06c31ea375cdd3e35058eb7bc93ae3df3a2e7))
- add production ready sets for trbfv and bfv
  ([#942](https://github.com/gnosisguild/enclave/issues/942))
  ([bdc1adf](https://github.com/gnosisguild/enclave/commit/bdc1adf12e23e888ffa810b1af6e0827155f2e1b))
- add support for dnsaddr resolution ([#1060](https://github.com/gnosisguild/enclave/issues/1060))
  ([d822758](https://github.com/gnosisguild/enclave/commit/d8227583da51ce34829af71494c5d3aa33a05f78))
- add trbfv actor test ([#660](https://github.com/gnosisguild/enclave/issues/660))
  ([3dd1a51](https://github.com/gnosisguild/enclave/commit/3dd1a5136e15fbb2ae39faeb7402c105955467e6))
- add vote validation and encoding in ts ([#848](https://github.com/gnosisguild/enclave/issues/848))
  ([914948d](https://github.com/gnosisguild/enclave/commit/914948d0a299bbae60f0e0232e228c2a14b713cb))
- add zk-inputs-wasm crate [skip-line-limit]
  ([#905](https://github.com/gnosisguild/enclave/issues/905))
  ([f2463fa](https://github.com/gnosisguild/enclave/commit/f2463fa46f7c3190c3083b387abb79bdc3894ff0))
- assign voter slot ([#843](https://github.com/gnosisguild/enclave/issues/843))
  ([4637a78](https://github.com/gnosisguild/enclave/commit/4637a785b9284e7442e598c3c2a0306a401acbe5))
- automatically config enclave.config.yaml
  ([#1014](https://github.com/gnosisguild/enclave/issues/1014))
  ([4c468fc](https://github.com/gnosisguild/enclave/commit/4c468fcca266a911cc0197e6181d1edbbdae1f51))
- bonsai to boundless migration [skip-line-limit]
  ([#1030](https://github.com/gnosisguild/enclave/issues/1030))
  ([6fd1668](https://github.com/gnosisguild/enclave/commit/6fd1668ebbd82435843847030b37fe6b18130b63))
- census tree on CRISP ([#763](https://github.com/gnosisguild/enclave/issues/763))
  ([ecd0ac2](https://github.com/gnosisguild/enclave/commit/ecd0ac23c8e18a8c2768992adfa6d8bb96740a0e)),
  closes [#779](https://github.com/gnosisguild/enclave/issues/779)
- ciphernode economic contracts [skip-line-limit]
  ([#766](https://github.com/gnosisguild/enclave/issues/766))
  ([c478909](https://github.com/gnosisguild/enclave/commit/c478909bd8aedebf93a3223dcbe91d85fceceb63))
- connect crisp to blockchain time [skip-line-limit]
  ([#1052](https://github.com/gnosisguild/enclave/issues/1052))
  ([9ac6408](https://github.com/gnosisguild/enclave/commit/9ac64085f276ab622ba572e2d7f21f372a838efb))
- crisp use param set 512_10_1 ([#1009](https://github.com/gnosisguild/enclave/issues/1009))
  ([5b72042](https://github.com/gnosisguild/enclave/commit/5b72042f544d85e953000942991692b41def7460))
- decode tally in ts ([#852](https://github.com/gnosisguild/enclave/issues/852))
  ([0a96b8e](https://github.com/gnosisguild/enclave/commit/0a96b8e20445364f01f2551e0a9f494bc33ad79c))
- deploy transparent proxy contracts [skip-line-limit]
  ([#987](https://github.com/gnosisguild/enclave/issues/987))
  ([b6f9b7b](https://github.com/gnosisguild/enclave/commit/b6f9b7ba71efa419902a0e707f93a2f9b150d6ea))
- deploy with hardhat in CRISP ([#875](https://github.com/gnosisguild/enclave/issues/875))
  ([f1567b8](https://github.com/gnosisguild/enclave/commit/f1567b8308d6468e6ec380535448d616d37be82b))
- do not use external input validator for programs [skip-line-limit]
  ([#996](https://github.com/gnosisguild/enclave/issues/996))
  ([9aa1e30](https://github.com/gnosisguild/enclave/commit/9aa1e30e7685236baf04663e5a6e8c8a4100d335))
- enclave start --experimental-trbfv and use ciphernodebuilder
  ([#856](https://github.com/gnosisguild/enclave/issues/856))
  ([d135c82](https://github.com/gnosisguild/enclave/commit/d135c8234661ee4b13806b076a450d8354316e38))
- encrypt vote and generate initial inputs
  ([#872](https://github.com/gnosisguild/enclave/issues/872))
  ([bab94ad](https://github.com/gnosisguild/enclave/commit/bab94ad663982a81c2c1156db23faca31ae7f209))
- expose params via wasm ([#993](https://github.com/gnosisguild/enclave/issues/993))
  ([e9e3590](https://github.com/gnosisguild/enclave/commit/e9e35909f06753e5e9b65030b25c43915423d12a))
- fetch round data from crisp server ([#811](https://github.com/gnosisguild/enclave/issues/811))
  ([0b305d1](https://github.com/gnosisguild/enclave/commit/0b305d1474073788c84d2cd0394c432c86d80499))
- fetch token data data from crisp server
  ([#804](https://github.com/gnosisguild/enclave/issues/804))
  ([4bac0c4](https://github.com/gnosisguild/enclave/commit/4bac0c4f5c6d6be1e62c6062baaa396895c9aec6))
- fetch token holders with etherscan api [skip-line-limit]
  ([#929](https://github.com/gnosisguild/enclave/issues/929))
  ([41abd8e](https://github.com/gnosisguild/enclave/commit/41abd8e75d8dc9639818b9bd710ddb7498f78d0f))
- fix infrastructure and prefactor net interface
  ([#903](https://github.com/gnosisguild/enclave/issues/903))
  ([b4610a8](https://github.com/gnosisguild/enclave/commit/b4610a8aacf1b786cd70cef641be941c4f0ba31f))
- generate merkle tree ([#826](https://github.com/gnosisguild/enclave/issues/826))
  ([d471fa5](https://github.com/gnosisguild/enclave/commit/d471fa546ac00ba23c3edd9d2e2b30d35856483a))
- greco gamma optimization ([#911](https://github.com/gnosisguild/enclave/issues/911))
  ([73b7ad4](https://github.com/gnosisguild/enclave/commit/73b7ad41de7ee1c0bfefc8f5fa3aa860e6a581f5))
- greco, e0 == e0is[i] check ([#1049](https://github.com/gnosisguild/enclave/issues/1049))
  ([237746a](https://github.com/gnosisguild/enclave/commit/237746aad8e2d667a33d0a4090d541b83d10b08e))
- inclusion proof ([#846](https://github.com/gnosisguild/enclave/issues/846))
  ([ba69679](https://github.com/gnosisguild/enclave/commit/ba696790f72c6f8c345b62f21c30081c85778bf3))
- indexer refactor - consolidate listeners and add ctx
  ([#1043](https://github.com/gnosisguild/enclave/issues/1043))
  ([eecd272](https://github.com/gnosisguild/enclave/commit/eecd2726aa075197a068494ba3a991cf19f9a20e))
- kademlia dht publishing: receiving document
  ([#828](https://github.com/gnosisguild/enclave/issues/828))
  ([84ed064](https://github.com/gnosisguild/enclave/commit/84ed06458eba18e94cfe08bd3af6bb77e714885a))
- limit PRs to 700 lines ([#821](https://github.com/gnosisguild/enclave/issues/821))
  ([26cd4a9](https://github.com/gnosisguild/enclave/commit/26cd4a985e82346182259b5d1e761ae9e5effb08))
- mask vote utilities ([#924](https://github.com/gnosisguild/enclave/issues/924))
  ([8d9d326](https://github.com/gnosisguild/enclave/commit/8d9d326c8da9ac4366d2f5ebd2bf210b2fedb005))
- multithread enable threadpool ([#1016](https://github.com/gnosisguild/enclave/issues/1016))
  ([7048a47](https://github.com/gnosisguild/enclave/commit/7048a473181a691cba07ec747bafa65825a2aca4))
- optimization by concatenating coefficients
  ([#734](https://github.com/gnosisguild/enclave/issues/734))
  ([00e2f6d](https://github.com/gnosisguild/enclave/commit/00e2f6d5eaaf2089488f414dc57675f7120cf2a0))
- prefactor for sync mode tidy up event structure [skip-line-limit]
  ([#1056](https://github.com/gnosisguild/enclave/issues/1056))
  ([9ddff8c](https://github.com/gnosisguild/enclave/commit/9ddff8ccb72d0917696e44d22bedc416731c47b7))
- signature generation and parsing ([#914](https://github.com/gnosisguild/enclave/issues/914))
  ([31ad834](https://github.com/gnosisguild/enclave/commit/31ad8342a921775e644b038ec16b6983d48a555b))
- store merkle tree on program [skip-line-limit]
  ([#1027](https://github.com/gnosisguild/enclave/issues/1027))
  ([e1a8f22](https://github.com/gnosisguild/enclave/commit/e1a8f22f6796f3b8907fe5b59ec4ad28a2e15f6d))
- ticket score sortition ([#698](https://github.com/gnosisguild/enclave/issues/698))
  ([ba2d8ef](https://github.com/gnosisguild/enclave/commit/ba2d8ef88f7bdf3bacccf7f808dd89c9de04b8ca))
- trbfv integration test [skip-line-limit]
  ([#969](https://github.com/gnosisguild/enclave/issues/969))
  ([418e42f](https://github.com/gnosisguild/enclave/commit/418e42feceb5596a50f4f039f45d0f102fb7d50b))
- upgrade to hardhat v3 and configure repo
  ([#677](https://github.com/gnosisguild/enclave/issues/677))
  ([7ccf6fa](https://github.com/gnosisguild/enclave/commit/7ccf6fa4d62a972a4d2336bd436d71bbc9b54535))
- validate vote is <= balance ([#954](https://github.com/gnosisguild/enclave/issues/954))
  ([bd5d35a](https://github.com/gnosisguild/enclave/commit/bd5d35ad7e8ec2fea7e8fd677ac24bf615fadd6b))
- validate voting power ([#851](https://github.com/gnosisguild/enclave/issues/851))
  ([d6e04bb](https://github.com/gnosisguild/enclave/commit/d6e04bb560f588c9f125dfef67bf9d27736011e2))
- verify contracts on etherscan ([#867](https://github.com/gnosisguild/enclave/issues/867))
  ([bd8d5bc](https://github.com/gnosisguild/enclave/commit/bd8d5bc95475859a42f5a55e37aa8581d4885df6))

## [0.1.5](https://github.com/gnosisguild/enclave/compare/v0.1.2...v0.1.5) (2025-10-13)

### Bug Fixes

- add logging on writing ([#842](https://github.com/gnosisguild/enclave/issues/842))
  ([679020e](https://github.com/gnosisguild/enclave/commit/679020e4183696eceddb0abf7e75931915c691e3))
- contracts exports ([#732](https://github.com/gnosisguild/enclave/issues/732))
  ([c0686c6](https://github.com/gnosisguild/enclave/commit/c0686c6b42b351c07adf400c47d8cc5b2573f8e6))
- ensure we don't have uncommited files ([#676](https://github.com/gnosisguild/enclave/issues/676))
  ([a46e707](https://github.com/gnosisguild/enclave/commit/a46e70795655b8ff3a9896651f09f5ccee2592c7))
- ensure we push to the correct branch ([#770](https://github.com/gnosisguild/enclave/issues/770))
  ([9d630a6](https://github.com/gnosisguild/enclave/commit/9d630a6eab7c2329eb4603e1bebe48a82b35adcc))
- fix reentrancy issue in enclave contracts
  ([#752](https://github.com/gnosisguild/enclave/issues/752))
  ([3806a87](https://github.com/gnosisguild/enclave/commit/3806a870b39fa47a1b4b77f9484c0a1d74bfbaa4))
- release rust crates error ([#689](https://github.com/gnosisguild/enclave/issues/689))
  ([3c25929](https://github.com/gnosisguild/enclave/commit/3c25929f2317003c81d3a21d6b4fc9b1e44573cc))
- remove already published files from gitignore
  ([#680](https://github.com/gnosisguild/enclave/issues/680))
  ([283205d](https://github.com/gnosisguild/enclave/commit/283205dffc665d83cc741c07f697c1ecaf2d1d84))
- remove ci artifacts deep clean ([#681](https://github.com/gnosisguild/enclave/issues/681))
  ([242aac9](https://github.com/gnosisguild/enclave/commit/242aac96b9800043b0d24b5716b3262baefd4472))
- remove guidance on typos and small changes
  ([#824](https://github.com/gnosisguild/enclave/issues/824))
  ([d2e083a](https://github.com/gnosisguild/enclave/commit/d2e083a8126b879ffd508571c4dd2985543dfcf4))
- remove mac intel build and allow crates publishing to fail
  ([#777](https://github.com/gnosisguild/enclave/issues/777))
  ([8025c27](https://github.com/gnosisguild/enclave/commit/8025c277d5c4aa1005ab93d84d34158266458800))
- rust crate release error ([#694](https://github.com/gnosisguild/enclave/issues/694))
  ([56e9b12](https://github.com/gnosisguild/enclave/commit/56e9b12c2b319d1ea1081df4577b6b0cd0ccfc7d))
- rust crates release workflow ([#715](https://github.com/gnosisguild/enclave/issues/715))
  ([fc330c6](https://github.com/gnosisguild/enclave/commit/fc330c625742bce01def98ef3ccec5ae15fbdb96))
- rust releases ([#774](https://github.com/gnosisguild/enclave/issues/774))
  ([16870a4](https://github.com/gnosisguild/enclave/commit/16870a42973fccae7a376ccbc4b952f9e971fffa))
- set correct mining settings for dev env
  ([#838](https://github.com/gnosisguild/enclave/issues/838))
  ([4a9ffdd](https://github.com/gnosisguild/enclave/commit/4a9ffdd8e31187be07f1e17cdfcf20b7ddf9d4bb))
- update relative paths to use git ([#708](https://github.com/gnosisguild/enclave/issues/708))
  ([e0bd2bc](https://github.com/gnosisguild/enclave/commit/e0bd2bc7a5e2515013188fc7e40927630d1f6d58))
- update viem version ([#844](https://github.com/gnosisguild/enclave/issues/844))
  ([e4c1a6b](https://github.com/gnosisguild/enclave/commit/e4c1a6ba225950eafb781cf704a4d674949312ce))
- wallet tx nonce & contract deployment ([#836](https://github.com/gnosisguild/enclave/issues/836))
  ([0141323](https://github.com/gnosisguild/enclave/commit/0141323098ddb7b00e447fb7d8f2aa94ee37f144))
- wasm init ([#740](https://github.com/gnosisguild/enclave/issues/740))
  ([58f7905](https://github.com/gnosisguild/enclave/commit/58f7905dd5bd33070be84b0bd5d88b5f44d98267))

### Features

- add a function to get an e3 public key ([#760](https://github.com/gnosisguild/enclave/issues/760))
  ([4db5dac](https://github.com/gnosisguild/enclave/commit/4db5dacf2f60872cfbafa16728b3da4f9244c248))
- add version flag ([#800](https://github.com/gnosisguild/enclave/issues/800))
  ([fe95d8c](https://github.com/gnosisguild/enclave/commit/fe95d8ccbeec828a0c4952a1111811bf6e7c2ef1))
- optimization by concatenating coefficients
  ([#734](https://github.com/gnosisguild/enclave/issues/734))
  ([00e2f6d](https://github.com/gnosisguild/enclave/commit/00e2f6d5eaaf2089488f414dc57675f7120cf2a0))
- upgrade to hardhat v3 and configure repo
  ([#677](https://github.com/gnosisguild/enclave/issues/677))
  ([7ccf6fa](https://github.com/gnosisguild/enclave/commit/7ccf6fa4d62a972a4d2336bd436d71bbc9b54535))

## [0.1.4](https://github.com/gnosisguild/enclave/compare/v0.1.2...v0.1.4) (2025-10-07)

### Bug Fixes

- contracts exports ([#732](https://github.com/gnosisguild/enclave/issues/732))
  ([c0686c6](https://github.com/gnosisguild/enclave/commit/c0686c6b42b351c07adf400c47d8cc5b2573f8e6))
- ensure we don't have uncommited files ([#676](https://github.com/gnosisguild/enclave/issues/676))
  ([a46e707](https://github.com/gnosisguild/enclave/commit/a46e70795655b8ff3a9896651f09f5ccee2592c7))
- ensure we push to the correct branch ([#770](https://github.com/gnosisguild/enclave/issues/770))
  ([9d630a6](https://github.com/gnosisguild/enclave/commit/9d630a6eab7c2329eb4603e1bebe48a82b35adcc))
- fix reentrancy issue in enclave contracts
  ([#752](https://github.com/gnosisguild/enclave/issues/752))
  ([3806a87](https://github.com/gnosisguild/enclave/commit/3806a870b39fa47a1b4b77f9484c0a1d74bfbaa4))
- release rust crates error ([#689](https://github.com/gnosisguild/enclave/issues/689))
  ([3c25929](https://github.com/gnosisguild/enclave/commit/3c25929f2317003c81d3a21d6b4fc9b1e44573cc))
- remove already published files from gitignore
  ([#680](https://github.com/gnosisguild/enclave/issues/680))
  ([283205d](https://github.com/gnosisguild/enclave/commit/283205dffc665d83cc741c07f697c1ecaf2d1d84))
- remove ci artifacts deep clean ([#681](https://github.com/gnosisguild/enclave/issues/681))
  ([242aac9](https://github.com/gnosisguild/enclave/commit/242aac96b9800043b0d24b5716b3262baefd4472))
- remove mac intel build and allow crates publishing to fail
  ([#777](https://github.com/gnosisguild/enclave/issues/777))
  ([8025c27](https://github.com/gnosisguild/enclave/commit/8025c277d5c4aa1005ab93d84d34158266458800))
- rust crate release error ([#694](https://github.com/gnosisguild/enclave/issues/694))
  ([56e9b12](https://github.com/gnosisguild/enclave/commit/56e9b12c2b319d1ea1081df4577b6b0cd0ccfc7d))
- rust crates release workflow ([#715](https://github.com/gnosisguild/enclave/issues/715))
  ([fc330c6](https://github.com/gnosisguild/enclave/commit/fc330c625742bce01def98ef3ccec5ae15fbdb96))
- rust releases ([#774](https://github.com/gnosisguild/enclave/issues/774))
  ([16870a4](https://github.com/gnosisguild/enclave/commit/16870a42973fccae7a376ccbc4b952f9e971fffa))
- update relative paths to use git ([#708](https://github.com/gnosisguild/enclave/issues/708))
  ([e0bd2bc](https://github.com/gnosisguild/enclave/commit/e0bd2bc7a5e2515013188fc7e40927630d1f6d58))
- wasm init ([#740](https://github.com/gnosisguild/enclave/issues/740))
  ([58f7905](https://github.com/gnosisguild/enclave/commit/58f7905dd5bd33070be84b0bd5d88b5f44d98267))

### Features

- add a function to get an e3 public key ([#760](https://github.com/gnosisguild/enclave/issues/760))
  ([4db5dac](https://github.com/gnosisguild/enclave/commit/4db5dacf2f60872cfbafa16728b3da4f9244c248))
- add version flag ([#800](https://github.com/gnosisguild/enclave/issues/800))
  ([fe95d8c](https://github.com/gnosisguild/enclave/commit/fe95d8ccbeec828a0c4952a1111811bf6e7c2ef1))
- optimization by concatenating coefficients
  ([#734](https://github.com/gnosisguild/enclave/issues/734))
  ([00e2f6d](https://github.com/gnosisguild/enclave/commit/00e2f6d5eaaf2089488f414dc57675f7120cf2a0))
- upgrade to hardhat v3 and configure repo
  ([#677](https://github.com/gnosisguild/enclave/issues/677))
  ([7ccf6fa](https://github.com/gnosisguild/enclave/commit/7ccf6fa4d62a972a4d2336bd436d71bbc9b54535))

## [0.1.3](https://github.com/gnosisguild/enclave/compare/v0.1.2...v0.1.3) (2025-10-02)

### Bug Fixes

- contracts exports ([#732](https://github.com/gnosisguild/enclave/issues/732))
  ([c0686c6](https://github.com/gnosisguild/enclave/commit/c0686c6b42b351c07adf400c47d8cc5b2573f8e6))
- ensure we don't have uncommited files ([#676](https://github.com/gnosisguild/enclave/issues/676))
  ([a46e707](https://github.com/gnosisguild/enclave/commit/a46e70795655b8ff3a9896651f09f5ccee2592c7))
- ensure we push to the correct branch ([#770](https://github.com/gnosisguild/enclave/issues/770))
  ([9d630a6](https://github.com/gnosisguild/enclave/commit/9d630a6eab7c2329eb4603e1bebe48a82b35adcc))
- fix reentrancy issue in enclave contracts
  ([#752](https://github.com/gnosisguild/enclave/issues/752))
  ([3806a87](https://github.com/gnosisguild/enclave/commit/3806a870b39fa47a1b4b77f9484c0a1d74bfbaa4))
- release rust crates error ([#689](https://github.com/gnosisguild/enclave/issues/689))
  ([3c25929](https://github.com/gnosisguild/enclave/commit/3c25929f2317003c81d3a21d6b4fc9b1e44573cc))
- remove already published files from gitignore
  ([#680](https://github.com/gnosisguild/enclave/issues/680))
  ([283205d](https://github.com/gnosisguild/enclave/commit/283205dffc665d83cc741c07f697c1ecaf2d1d84))
- remove ci artifacts deep clean ([#681](https://github.com/gnosisguild/enclave/issues/681))
  ([242aac9](https://github.com/gnosisguild/enclave/commit/242aac96b9800043b0d24b5716b3262baefd4472))
- remove mac intel build and allow crates publishing to fail
  ([#777](https://github.com/gnosisguild/enclave/issues/777))
  ([8025c27](https://github.com/gnosisguild/enclave/commit/8025c277d5c4aa1005ab93d84d34158266458800))
- rust crate release error ([#694](https://github.com/gnosisguild/enclave/issues/694))
  ([56e9b12](https://github.com/gnosisguild/enclave/commit/56e9b12c2b319d1ea1081df4577b6b0cd0ccfc7d))
- rust crates release workflow ([#715](https://github.com/gnosisguild/enclave/issues/715))
  ([fc330c6](https://github.com/gnosisguild/enclave/commit/fc330c625742bce01def98ef3ccec5ae15fbdb96))
- rust releases ([#774](https://github.com/gnosisguild/enclave/issues/774))
  ([16870a4](https://github.com/gnosisguild/enclave/commit/16870a42973fccae7a376ccbc4b952f9e971fffa))
- update relative paths to use git ([#708](https://github.com/gnosisguild/enclave/issues/708))
  ([e0bd2bc](https://github.com/gnosisguild/enclave/commit/e0bd2bc7a5e2515013188fc7e40927630d1f6d58))
- wasm init ([#740](https://github.com/gnosisguild/enclave/issues/740))
  ([58f7905](https://github.com/gnosisguild/enclave/commit/58f7905dd5bd33070be84b0bd5d88b5f44d98267))

### Features

- add a function to get an e3 public key ([#760](https://github.com/gnosisguild/enclave/issues/760))
  ([4db5dac](https://github.com/gnosisguild/enclave/commit/4db5dacf2f60872cfbafa16728b3da4f9244c248))
- optimization by concatenating coefficients
  ([#734](https://github.com/gnosisguild/enclave/issues/734))
  ([00e2f6d](https://github.com/gnosisguild/enclave/commit/00e2f6d5eaaf2089488f414dc57675f7120cf2a0))
- upgrade to hardhat v3 and configure repo
  ([#677](https://github.com/gnosisguild/enclave/issues/677))
  ([7ccf6fa](https://github.com/gnosisguild/enclave/commit/7ccf6fa4d62a972a4d2336bd436d71bbc9b54535))

## [0.0.14-test](https://github.com/gnosisguild/enclave/compare/v0.1.2...v0.0.14-test) (2025-10-01)

### Bug Fixes

- contracts exports ([#732](https://github.com/gnosisguild/enclave/issues/732))
  ([c0686c6](https://github.com/gnosisguild/enclave/commit/c0686c6b42b351c07adf400c47d8cc5b2573f8e6))
- ensure we don't have uncommited files ([#676](https://github.com/gnosisguild/enclave/issues/676))
  ([a46e707](https://github.com/gnosisguild/enclave/commit/a46e70795655b8ff3a9896651f09f5ccee2592c7))
- ensure we update the cargo crates too
  ([94f5231](https://github.com/gnosisguild/enclave/commit/94f52319cd2f3c06ad1b0428c58ff95e0ae40c63))
- fix reentrancy issue in enclave contracts
  ([#752](https://github.com/gnosisguild/enclave/issues/752))
  ([3806a87](https://github.com/gnosisguild/enclave/commit/3806a870b39fa47a1b4b77f9484c0a1d74bfbaa4))
- release rust crates error ([#689](https://github.com/gnosisguild/enclave/issues/689))
  ([3c25929](https://github.com/gnosisguild/enclave/commit/3c25929f2317003c81d3a21d6b4fc9b1e44573cc))
- remove already published files from gitignore
  ([#680](https://github.com/gnosisguild/enclave/issues/680))
  ([283205d](https://github.com/gnosisguild/enclave/commit/283205dffc665d83cc741c07f697c1ecaf2d1d84))
- remove ci artifacts deep clean ([#681](https://github.com/gnosisguild/enclave/issues/681))
  ([242aac9](https://github.com/gnosisguild/enclave/commit/242aac96b9800043b0d24b5716b3262baefd4472))
- remove dprint in favour of cargo fmt
  ([412fa9b](https://github.com/gnosisguild/enclave/commit/412fa9be525672449394e41a216666a56a7821a1))
- rust crate release error ([#694](https://github.com/gnosisguild/enclave/issues/694))
  ([56e9b12](https://github.com/gnosisguild/enclave/commit/56e9b12c2b319d1ea1081df4577b6b0cd0ccfc7d))
- rust crates release workflow ([#715](https://github.com/gnosisguild/enclave/issues/715))
  ([fc330c6](https://github.com/gnosisguild/enclave/commit/fc330c625742bce01def98ef3ccec5ae15fbdb96))
- update relative paths to use git ([#708](https://github.com/gnosisguild/enclave/issues/708))
  ([e0bd2bc](https://github.com/gnosisguild/enclave/commit/e0bd2bc7a5e2515013188fc7e40927630d1f6d58))
- wasm init ([#740](https://github.com/gnosisguild/enclave/issues/740))
  ([58f7905](https://github.com/gnosisguild/enclave/commit/58f7905dd5bd33070be84b0bd5d88b5f44d98267))

### Features

- add a function to get an e3 public key ([#760](https://github.com/gnosisguild/enclave/issues/760))
  ([4db5dac](https://github.com/gnosisguild/enclave/commit/4db5dacf2f60872cfbafa16728b3da4f9244c248))
- add changelog feature to bump script
  ([319ef67](https://github.com/gnosisguild/enclave/commit/319ef6795e4846a89d04f526d24a2c15bd37915d))
- add script to bump versions and bump to 0.0.15-test
  ([aada549](https://github.com/gnosisguild/enclave/commit/aada549f45ef35803a3dbde46c574787db7c5215))
- optimization by concatenating coefficients
  ([#734](https://github.com/gnosisguild/enclave/issues/734))
  ([00e2f6d](https://github.com/gnosisguild/enclave/commit/00e2f6d5eaaf2089488f414dc57675f7120cf2a0))
- unify releases
  ([820ea9d](https://github.com/gnosisguild/enclave/commit/820ea9d35a25286610a1e71a6a5d7d3b15079679))
- update bump script to also push to git
  ([49a35f7](https://github.com/gnosisguild/enclave/commit/49a35f722c33e9f41d9052c35a64816f09f45342))
- upgrade to hardhat v3 and configure repo
  ([#677](https://github.com/gnosisguild/enclave/issues/677))
  ([7ccf6fa](https://github.com/gnosisguild/enclave/commit/7ccf6fa4d62a972a4d2336bd436d71bbc9b54535))
