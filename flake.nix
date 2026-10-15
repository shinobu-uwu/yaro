{
  description = "Yaro kernel and development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    pwndbg.url = "github:pwndbg/pwndbg";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    pwndbg,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [rust-overlay.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;
        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            ./.cargo
            # Include all workspace crates and their build assets automatically.
            ./crates
          ];
        };

        commonArgs = {
          inherit src;
          pname = "yaro";
          strictDeps = true;
          doCheck = false;

          buildInputs =
            [
            ]
            ++ lib.optionals pkgs.stdenv.isDarwin [
              pkgs.libiconv
            ];
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs =
          commonArgs
          // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          };

        kernel = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "yaro_kernel";
            cargoExtraArgs = "--locked -p yaro_kernel";
          }
        );

        limine = pkgs.fetchFromGitHub {
          owner = "limine-bootloader";
          repo = "limine";
          # v10.8.3-binary; native setup fetches this same commit.
          rev = lib.trim (builtins.readFile ./limine.rev);
          hash = "sha256-5dK7EFLrCd+JEZg3WTaHvY+osfDeUlKbXFv7oHAcDtE=";
        };

        limineInstaller = pkgs.stdenv.mkDerivation {
          pname = "limine-installer";
          version = "10.8.3";
          src = limine;
          dontConfigure = true;

          buildPhase = ''
            runHook preBuild
            "$CC" -O2 -std=c99 limine.c -o limine
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            install -Dm755 limine "$out/bin/limine"
            runHook postInstall
          '';
        };

        devTools = [pkgs.just pkgs.xorriso pkgs.qemu];
        devEnv = {
          LIMINE_DIR = "${limine}";
          LIMINE = "${limineInstaller}/bin/limine";
          OVMF_CODE = "${pkgs.OVMF.fd}/FV/OVMF_CODE.fd";
          OVMF_VARS = "${pkgs.OVMF.fd}/FV/OVMF_VARS.fd";
        };

        iso = pkgs.stdenv.mkDerivation {
          pname = "kernel-iso";
          version = "0.1.0";

          nativeBuildInputs = [
            pkgs.just
            pkgs.bash
            pkgs.xorriso
          ];

          dontUnpack = true;

          buildPhase = ''
            runHook preBuild
            just --justfile ${./justfile} --working-directory "$PWD" package-iso \
              ${kernel}/bin/yaro_kernel ${limine} ${./limine.conf} kernel.iso \
              ${limineInstaller}/bin/limine
            runHook postBuild
          '';

          installPhase = ''
            runHook preInstall
            install -Dm644 kernel.iso "$out/kernel.iso"
            runHook postInstall
          '';
        };
      in {
        packages = {
          inherit kernel;
          default = kernel;
          iso = iso;
        };
        apps = rec {
          default = dev;
          dev = {
            type = "app";
            program = lib.getExe (pkgs.writeShellApplication {
              name = "yaro-dev";
              runtimeInputs = [rustToolchain pkgs.stdenv.cc pkgs.coreutils pkgs.bash] ++ devTools;
              text = ''
                if [[ ! -f crates/kernel/Cargo.toml || ! -f justfile ]]; then
                  echo "Run nix run from the repository root." >&2
                  exit 1
                fi

                export LIMINE_DIR=${devEnv.LIMINE_DIR}
                export LIMINE=${devEnv.LIMINE}
                export OVMF_CODE="''${OVMF_CODE:-${devEnv.OVMF_CODE}}"
                export OVMF_VARS="''${OVMF_VARS:-${devEnv.OVMF_VARS}}"
                exec just run "$@"
              '';
            });
          };
        };
        devShells.default = craneLib.devShell (devEnv
          // {
            packages = [pkgs.gdb] ++ devTools;
          });
      }
    );
}
