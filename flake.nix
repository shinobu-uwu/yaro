{
  description = "Limine rust template for nix";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = {
    nixpkgs,
    crane,
    flake-utils,
    rust-overlay,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [rust-overlay.overlays.default];
        pkgs = import nixpkgs {inherit system overlays;};

        inherit (pkgs) lib;

        rustToolchain = pkgs.rust-bin.nightly.latest.default.override {
          targets = ["x86_64-unknown-none"];
          extensions = [
            # includes already:
            # rustc
            # cargo
            # rust-std
            # rust-docs
            # rustfmt-preview
            # clippy-preview
            "rust-analyzer"
            "rust-src"
          ];
        };
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

          CARGO_BUILD_TARGET = "x86_64-unknown-none";
          RUSTFLAGS = "-C relocation-model=static";
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
            pname = "kernel";
            cargoExtraArgs = "-p kernel";
          }
        );

        limine = pkgs.fetchFromGitHub {
          owner = "limine-bootloader";
          repo = "limine";
          rev = "v10.8.3-binary";
          hash = "sha256-5dK7EFLrCd+JEZg3WTaHvY+osfDeUlKbXFv7oHAcDtE=";
        };

        iso = pkgs.stdenv.mkDerivation {
          pname = "kernel-iso";
          version = "0.1.0";

          nativeBuildInputs = [
            pkgs.xorriso
            pkgs.limine
          ];

          dontUnpack = true;

          buildPhase = ''
            mkdir -p iso_root/boot/limine
            mkdir -p iso_root/EFI/BOOT

            cp ${kernel}/bin/kernel iso_root/boot/kernel

            cp ${./limine.conf} iso_root/boot/limine/limine.conf

            cp ${limine}/limine-bios.sys iso_root/boot/limine/limine-bios.sys
            cp ${limine}/limine-bios-cd.bin iso_root/boot/limine/limine-bios-cd.bin
            cp ${limine}/limine-uefi-cd.bin iso_root/boot/limine/limine-uefi-cd.bin

            cp ${limine}/BOOTX64.EFI iso_root/EFI/BOOT/BOOTX64.EFI
            cp ${limine}/BOOTIA32.EFI iso_root/EFI/BOOT/BOOTIA32.EFI
          '';

          installPhase = ''
            mkdir -p $out


            xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
            -no-emul-boot -boot-load-size 4 -boot-info-table \
            --efi-boot boot/limine/limine-uefi-cd.bin \
            -efi-boot-part --efi-boot-image --protective-msdos-label \
            iso_root -o $out/kernel.iso

            ${pkgs.limine}/bin/limine bios-install $out/kernel.iso
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
              runtimeInputs = [rustToolchain pkgs.stdenv.cc pkgs.coreutils pkgs.xorriso pkgs.limine pkgs.qemu];
              text = ''
                if [[ ! -f crates/kernel/Cargo.toml || ! -f limine.conf ]]; then
                  echo "Run nix run from the repository root." >&2
                  exit 1
                fi

                # Keep Cargo artifacts in the working tree across invocations.
                cargo build --locked -p kernel --target x86_64-unknown-none --target-dir "$PWD/target"

                run_dir=$(mktemp -d "$PWD/target/qemu-dev.XXXXXX")
                trap 'rm -rf "$run_dir"' EXIT
                iso_root="$run_dir/iso_root"
                mkdir -p "$iso_root/boot/limine" "$iso_root/EFI/BOOT"
                cp target/x86_64-unknown-none/debug/kernel "$iso_root/boot/kernel"
                cp limine.conf "$iso_root/boot/limine/limine.conf"
                cp ${limine}/limine-bios.sys ${limine}/limine-bios-cd.bin \
                  ${limine}/limine-uefi-cd.bin "$iso_root/boot/limine/"
                cp ${limine}/BOOTX64.EFI ${limine}/BOOTIA32.EFI "$iso_root/EFI/BOOT/"

                xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
                  -no-emul-boot -boot-load-size 4 -boot-info-table \
                  --efi-boot boot/limine/limine-uefi-cd.bin \
                  -efi-boot-part --efi-boot-image --protective-msdos-label \
                  "$iso_root" -o "$run_dir/kernel.iso"
                limine bios-install "$run_dir/kernel.iso"

                cp ${pkgs.OVMF.fd}/FV/OVMF_VARS.fd "$run_dir/OVMF_VARS.fd"
                chmod u+w "$run_dir/OVMF_VARS.fd"
                qemu-system-x86_64 \
                  -M q35 \
                  -serial stdio \
                  -drive if=pflash,unit=0,format=raw,file=${pkgs.OVMF.fd}/FV/OVMF_CODE.fd,readonly=on \
                  -drive "if=pflash,unit=1,format=raw,file=$run_dir/OVMF_VARS.fd" \
                  -cdrom "$run_dir/kernel.iso" "$@"
              '';
            });
          };
        };
        devShells.default = craneLib.devShell {
          packages = [
            # any package that might be useful for development, suchs as
            # gdb
            # bintools
          ];
        };
      }
    );
}
