{
  description = "e3-cli";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages.${system};

        e3-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "e3-cli";
          version = (builtins.fromTOML (builtins.readFile (self + "/crates/cli/Cargo.toml"))).package.version;

          src = self;

          cargoLock.lockFile = self + "/Cargo.lock";

          buildAndTestSubdir = "crates/cli";

          nativeBuildInputs = [
            pkgs.pkg-config
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
