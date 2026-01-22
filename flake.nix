{
  description = "estrella";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
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
    (flake-utils.lib.eachDefaultSystem (
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

        # Frontend build
        frontendDeps = pkgs.buildNpmPackage {
          pname = "estrella-frontend";
          version = "0.1.0";
          src = ./frontend;
          npmDepsHash = "sha256-ElaO6LI7B0cGLbpSfMrIGxA8xIyUvPS79LPdjS3H6hs="; # Update after npm install
          buildPhase = "npm run build";
          installPhase = "cp -r dist $out";
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

          # Copy frontend build before cargo build
          preBuild = ''
            mkdir -p frontend/dist
            cp -r ${frontendDeps}/* frontend/dist/ || true
          '';
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
              # nodejs_24
              # nodePackages.npm
            ];
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
          '';
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath buildInputs;
        };
      }
    )) // {
      # Overlay to add estrella to pkgs
      overlays.default = final: prev: {
        estrella = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };

      # NixOS module for estrella HTTP server
      nixosModules.default = import ./nix/modules/estrella.nix;
      nixosModules.estrella = import ./nix/modules/estrella.nix;
    };
    # based on https://github.com/hiveboardgame/hive/blob/50b3804378012ee4ecf62f6e47ca348454eb066b/flake.nix
}
