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
          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "pdf417-0.3.0" = "sha256-9qyCrRWync4hasSblYJUBXBbbFvservSeXJOjOcu0r0=";
            };
          };
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
        #
        # Why musl? We need a single static binary with zero runtime deps
        # so it runs on any Raspberry Pi regardless of what's installed.
        # Building for aarch64-unknown-linux-musl produces that.
        #
        # Why Zig? Some Rust crates (e.g. TLS) contain C code that must be
        # compiled for musl. The "proper" way is a GCC cross-compiler
        # (aarch64-unknown-linux-musl-gcc), but nobody pre-builds that for
        # the Nix binary cache, so Nix would compile GCC from source (~20 min).
        # Zig ships a C compiler with musl built-in for every target, and IS
        # cached in nixpkgs. So we use `zig cc` as a drop-in musl CC instead.
        packages.static =
          let
            target = "aarch64-unknown-linux-musl";
            # Stable Rust with musl target (nixpkgs' Rust doesn't include it)
            rustToolchainMusl = pkgs.rust-bin.stable.latest.default.override {
              targets = [ target ];
            };
            muslRustPlatform = pkgs.makeRustPlatform {
              cargo = rustToolchainMusl;
              rustc = rustToolchainMusl;
            };
            zigCC = pkgs.writeShellScriptBin "zigcc" ''
              args=()
              for arg in "$@"; do
                case "$arg" in
                  --target=*) ;; # strip: cc crate passes --target=aarch64-unknown-linux-musl but Zig needs aarch64-linux-musl
                  *) args+=("$arg") ;;
                esac
              done
              exec ${pkgs.zig}/bin/zig cc -target aarch64-linux-musl "''${args[@]}"
            '';
          in
          muslRustPlatform.buildRustPackage {
            pname = "estrella-static";
            version = "0.1.0";
            src = ./.;
            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "pdf417-0.3.0" = "sha256-9qyCrRWync4hasSblYJUBXBbbFvservSeXJOjOcu0r0=";
              };
            };
            doCheck = false;
            auditable = false; # cargo-auditable passes --undefined which Zig's linker doesn't support

            nativeBuildInputs = [ pkgs.zig ];

            CC_aarch64_unknown_linux_musl = "${zigCC}/bin/zigcc";

            # Override build/install phases to explicitly pass --target,
            # because nixpkgs' cargo hooks ignore cargoBuildTarget in
            # non-cross builds and silently build for the native glibc target.
            buildPhase = ''
              export ZIG_GLOBAL_CACHE_DIR="$TMPDIR/zig-cache"
              mkdir -p "$ZIG_GLOBAL_CACHE_DIR"
              mkdir -p frontend/dist
              cp -r ${frontendDeps}/* frontend/dist/ || true
              cargo build -j $NIX_BUILD_CORES \
                --release --frozen --no-default-features \
                --target ${target}
            '';

            installPhase = ''
              mkdir -p $out/bin
              cp target/${target}/release/estrella $out/bin/
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
