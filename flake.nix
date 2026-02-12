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

        # Frontend build
        frontendDeps = pkgs.buildNpmPackage {
          pname = "estrella-frontend";
          version = "0.1.0";
          src = ./frontend;
          npmDepsHash = "sha256-MaPU2rEK6l+0yWFfvNa+fwKkm1EENfq77cr00EqiF4w="; # Update after npm install
          buildPhase = ''
            mkdir -p ../src/fixtures
            cp ${./src/fixtures/morning-briefing.json} ../src/fixtures/morning-briefing.json
            cp ${./src/fixtures/canvas-showcase.json} ../src/fixtures/canvas-showcase.json
            cp ${./src/fixtures/emoji-showcase.json} ../src/fixtures/emoji-showcase.json
            npm run build
          '';
          installPhase = "cp -r dist $out";
        };
      in
      with pkgs;
      {
        # Uses nixpkgs' cached Rust — no toolchain compilation needed
        packages.default = rustPlatform.buildRustPackage {
          pname = "estrella";
          version = "0.1.0";
          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = [ pkg-config ];
          buildInputs = [ libheif ];

          # Copy frontend build before cargo build
          preBuild = ''
            mkdir -p frontend/dist
            cp -r ${frontendDeps}/* frontend/dist/ || true
          '';
        };

        # Static musl build for .deb packaging (no libheif, no openssl)
        # Use `nix build .#static` on Linux ARM CI runners
        packages.static =
          let
            muslPkgs = pkgs.pkgsCross.aarch64-multiplatform-musl;
            muslCC = muslPkgs.stdenv.cc;
          in
          rustPlatform.buildRustPackage {
            pname = "estrella-static";
            version = "0.1.0";
            src = ./.;
            cargoLock.lockFile = ./Cargo.lock;
            buildNoDefaultFeatures = true;
            doCheck = false;

            depsBuildBuild = [ muslCC ];

            cargoBuildTarget = "aarch64-unknown-linux-musl";
            CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = "${muslCC}/bin/${muslCC.targetPrefix}cc";

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
              libheif
              gnupg
              nodejs
              playwright-test
              # Stable Rust from rust-overlay for dev (includes rust-analyzer)
              (rust-bin.stable.latest.default.override {
                extensions = [
                  "rust-src"
                  "rust-analyzer"
                ];
              })
            ];
          shellHook = ''
            export CARGO_TARGET_DIR="$PWD/.cargo/target"
            export GNUPGHOME="$PWD/.gnupg"
            mkdir -p "$GNUPGHOME"
            chmod 700 "$GNUPGHOME"
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
}
