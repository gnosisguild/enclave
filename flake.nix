{
  description = "Enclave";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }: let
    depLock = builtins.fromJSON (builtins.readFile ./versions/dep.lock.json);
    bbVersions = builtins.fromJSON (builtins.readFile ./versions/bb.versions.json);

    noirHash = "sha256-RoeWaqgFwr8A4HAlu5DzuxrNrexMolIZG14fHQA0KmM=";
    fheHash = "sha256-dS8LcKDI/D9ycsRXbQnMVkUc2ymFBFL8kDrEtRGuHNI=";
    vfsHash = "sha256-+d8RFk7UgOXDCE/LizCTV+UX/Xm/1mYWrR7W0l6mAl8=";
  in
    flake-utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages.${system};

      noirSrc = pkgs.fetchFromGitHub {
        owner = "noir-lang";
        repo = "noir";
        rev = "v1.0.0-beta.16";
        hash = noirHash;
      };

      mkBB = {version}: let
        hashes = bbVersions.${version} or (throw "Unknown bb version ${version}. Available: ${builtins.concatStringsSep ", " (builtins.attrNames bbVersions)}");
        bbPlatform =
          if pkgs.stdenv.isLinux
          then
            if pkgs.stdenv.isAarch64
            then "arm64-linux"
            else "amd64-linux"
          else if pkgs.stdenv.isDarwin
          then
            if pkgs.stdenv.isAarch64
            then "arm64-darwin"
            else "amd64-darwin"
          else throw "Unsupported platform";
        bb = pkgs.stdenv.mkDerivation {
          pname = "barretenberg";
          inherit version;
          src = pkgs.fetchurl {
            url = "https://github.com/AztecProtocol/aztec-packages/releases/download/v${version}/barretenberg-${bbPlatform}.tar.gz";
            sha256 = hashes.${bbPlatform} or (throw "No hash for ${bbPlatform} in version ${version}");
          };
          nativeBuildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [pkgs.autoPatchelfHook];
          buildInputs = pkgs.lib.optionals pkgs.stdenv.isLinux [pkgs.stdenv.cc.cc.lib];
          sourceRoot = ".";
          installPhase = ''
            mkdir -p $out/bin
            install -D -m755 bb $out/bin/bb
          '';
          meta = {
            description = "Barretenberg proving system";
            homepage = "https://github.com/AztecProtocol/aztec-packages";
          };
        };
      in
        if pkgs.stdenv.isLinux
        then
          pkgs.buildFHSEnv {
            name = "bb";
            targetPkgs = p: [bb p.stdenv.cc.cc.lib];
            runScript = "${bb}/bin/bb";
          }
        else bb;

      e3-cli = pkgs.rustPlatform.buildRustPackage {
        pname = "e3-cli";
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        src = ./.;
        GIT_COMMIT = "unknown";
        GIT_DIRTY = "false";
        preBuild = ''
          for d in $(find /build -type d -name 'noirc_driver*'); do
            if [ -d "$d/src" ]; then
              cp -r ${noirSrc}/noir_stdlib "$d/../../noir_stdlib"
            fi
          done

          export HOME=$(mktemp -d)
          git config --global user.email "nix@nix"
          git config --global user.name "nix"
          git init
          git add -A
          git commit -m "nix build" --allow-empty
        '';
        cargoLock = {
          lockFile = ./Cargo.lock;
          outputHashes = builtins.listToAttrs (
            map (name: {
              inherit name;
              value = noirHash;
            }) [
              "acir-1.0.0-beta.16"
              "acir_field-1.0.0-beta.16"
              "acvm-1.0.0-beta.16"
              "acvm_blackbox_solver-1.0.0-beta.16"
              "bn254_blackbox_solver-1.0.0-beta.16"
              "brillig-1.0.0-beta.16"
              "brillig_vm-1.0.0-beta.16"
              "fm-1.0.0-beta.16"
              "iter-extended-1.0.0-beta.16"
              "nargo-1.0.0-beta.16"
              "noir_greybox_fuzzer-1.0.0-beta.16"
              "noir_protobuf-1.0.0-beta.16"
              "noirc_abi-1.0.0-beta.16"
              "noirc_arena-1.0.0-beta.16"
              "noirc_artifacts-1.0.0-beta.16"
              "noirc_driver-1.0.0-beta.16"
              "noirc_errors-1.0.0-beta.16"
              "noirc_evaluator-1.0.0-beta.16"
              "noirc_frontend-1.0.0-beta.16"
              "noirc_printable_type-1.0.0-beta.16"
              "noirc_span-1.0.0-beta.16"
            ]
            ++ map (name: {
              inherit name;
              value = fheHash;
            }) [
              "fhe-0.1.0-beta.7"
              "fhe-math-0.1.0-beta.7"
              "fhe-traits-0.1.0-beta.7"
              "fhe-util-0.1.0-beta.7"
            ]
            ++ map (name: {
              inherit name;
              value = vfsHash;
            }) [
              "vfs-0.12.1"
            ]
          );
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

      mkE3Shell = {version}: let
        deps = depLock.${version} or (throw "Unknown e3 version ${version}. Available: ${builtins.concatStringsSep ", " (builtins.attrNames depLock)}");
        bb = mkBB {version = deps.bb;};
      in
        pkgs.mkShell {
          packages = [
            e3-cli
            bb
          ];
          shellHook = ''
            export E3_CUSTOM_BB="${bb}/bin/bb"
          '';
        };
    in {
      packages.default = e3-cli;
      packages.e3-cli = e3-cli;

      lib = {inherit mkBB mkE3Shell;};

      devShells.default = mkE3Shell {version = "0.1.14";};
    })
    // {
      templates.default = {
        path = ./template;
        description = "New project using e3 tools";
      };
    };
}
