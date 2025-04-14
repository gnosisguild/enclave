// Copyright 2024 RISC Zero, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

pragma solidity ^0.8.20;

import {RiscZeroCheats} from "risc0/test/RiscZeroCheats.sol";
import {console2} from "forge-std/console2.sol";
import {Test} from "forge-std/Test.sol";
import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {CRISPRisc0} from "../contracts/CRISPRisc0.sol";
import {Elf} from "./Elf.sol"; // auto-generated contract after running `cargo build`.

contract CRISPRisc0Test is RiscZeroCheats, Test {
    CRISPRisc0 public crispRisc0;

    // function setUp() public {
    //     IRiscZeroVerifier verifier = deployRiscZeroVerifier();
    //     crispRisc0 = new CRISPRisc0(verifier);
    //     assertEq(crispRisc0.owner(), address(this));
    // }

    // function test_SetEven() public {
    //     uint256 number = 12345678;
    //     (bytes memory journal, bytes memory seal) = prove(Elf.IS_EVEN_PATH, abi.encode(number));

    //     evenNumber.set(abi.decode(journal, (uint256)), seal);
    //     assertEq(evenNumber.get(), number);
    // }

    // function test_SetZero() public {
    //     uint256 number = 0;
    //     (bytes memory journal, bytes memory seal) = prove(Elf.IS_EVEN_PATH, abi.encode(number));

    //     evenNumber.set(abi.decode(journal, (uint256)), seal);
    //     assertEq(evenNumber.get(), number);
    // }
}
