{
  description = "e3-cli";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    nixpkgs,
    flake-utils,
    self,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};

        noirSrc = pkgs.fetchFromGitHub {
          owner = "noir-lang";
          repo = "noir";
          rev = "v1.0.0-beta.16";
          hash = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
        };

        e3-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "e3-cli";
          version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
          src = ./.;
          GIT_COMMIT = "unknown";
          GIT_DIRTY = "false";
          preBuild = ''
            # noirc_driver needs the real noir_stdlib
            for d in $(find /build -type d -name 'noirc_driver*'); do
              if [ -d "$d/src" ]; then
                cp -r ${noirSrc}/noir_stdlib "$d/../../noir_stdlib"
              fi
            done

            # build scripts need a git repo
            export HOME=$(mktemp -d)
            git config --global user.email "nix@nix"
            git config --global user.name "nix"
            git init
            git add -A
            git commit -m "nix build" --allow-empty
          '';
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "acir-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "acir_field-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "acvm-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "acvm_blackbox_solver-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "bn254_blackbox_solver-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "brillig-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "brillig_vm-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "fhe-0.1.0-beta.7" = "sha256-dS8LcKDI/D9ycsRXbQnMVkUc2ymFBFL8kDrEtRGuHNI=";
              "fhe-math-0.1.0-beta.7" = "sha256-dS8LcKDI/D9ycsRXbQnMVkUc2ymFBFL8kDrEtRGuHNI=";
              "fhe-traits-0.1.0-beta.7" = "sha256-dS8LcKDI/D9ycsRXbQnMVkUc2ymFBFL8kDrEtRGuHNI=";
              "fhe-util-0.1.0-beta.7" = "sha256-dS8LcKDI/D9ycsRXbQnMVkUc2ymFBFL8kDrEtRGuHNI=";
              "fm-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "iter-extended-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "nargo-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noir_greybox_fuzzer-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noir_protobuf-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_abi-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_arena-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_artifacts-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_driver-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_errors-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_evaluator-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_frontend-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_printable_type-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "noirc_span-1.0.0-beta.16" = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
              "vfs-0.12.1" = "sha256-+d8RFk7UgOXDCE/LizCTV+UX/Xm/1mYWrR7W0l6mAl8=";
            };
          };
          buildAndTestSubdir = "crates/cli";
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.git
            pkgs.pnpm
            pkgs.nodejs
            pkgs.jq
            pkgs.solc
          ];
          buildInputs =
            [
              pkgs.openssl
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.Security
              pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
            ];
          meta = {
            description = "e3 CLI";
            license = pkgs.lib.licenses.mit;
          };
        };
      in {
        packages.default = e3-cli;
        packages.e3-cli = e3-cli;
      }
    );
}
