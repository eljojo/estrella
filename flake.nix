{
  description = "estrella";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-25.11";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
    playwright = {
      url = "github:pietdevries94/playwright-web-flake";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-utils.follows = "flake-utils";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
      playwright,
      ...
    }:
    (flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [
          (import rust-overlay)
          (final: prev: {
            inherit (playwright.packages.${system}) playwright-test playwright-driver;
          })
        ];
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
          npmDepsHash = "sha256-DakZWPBlcfvcxJv0u3DFasjpG+V0M7CA+Cn+Iw0+tGo="; # Update after npm install
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
          buildInputs = [ openssl libheif ];

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
              libheif
              rustToolchain
              nodejs
              playwright-test
            ];
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
            export PLAYWRIGHT_BROWSERS_PATH="${pkgs.playwright-driver.browsers}"
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
