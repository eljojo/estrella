{
  description = "estrella";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        rustToolchain = pkgs.rust-bin.selectLatestNightlyWith (
          toolchain:
          toolchain.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
            ];
            targets = [ "wasm32-unknown-unknown" ];
          }
        );
        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };
      in
      with pkgs;
      {
        packages.default = rustPlatform.buildRustPackage {
          pname = "estrella";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkg-config ];
          buildInputs = [ openssl ];
        };

        devShells.default = mkShell rec {
          buildInputs =
            [
              pkg-config
              cacert
              cargo-make
              cargo
              rustfmt
              openssl
              rustToolchain
            ];
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            echo "Welcome to estrella"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    );
    # based on https://github.com/hiveboardgame/hive/blob/50b3804378012ee4ecf62f6e47ca348454eb066b/flake.nix
}
