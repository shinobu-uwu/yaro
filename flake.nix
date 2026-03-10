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

  outputs =
    {
      nixpkgs,
      crane,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit system overlays; };

        inherit (pkgs) lib;

        craneLib = (crane.mkLib pkgs).overrideToolchain (
          p:
          p.rust-bin.stable.latest.default.override {
            targets = [ "x86_64-unknown-none" ];
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
          }
        );
        src = craneLib.cleanCargoSource ./.;

        commonArgs = {
          inherit src;
          strictDeps = true;
          doCheck = false;

          buildInputs = [
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin [
            pkgs.libiconv
          ];

          CARGO_BUILD_TARGET = "x86_64-unknown-none";
          RUSTFLAGS = "-C relocation-model=static";
        };

        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
        };

        fileSetForCrate =
          crate:
          lib.fileset.toSource {
            root = ./.;
            fileset = lib.fileset.unions [
              ./Cargo.toml
              ./Cargo.lock
              (craneLib.fileset.commonCargoSources crate)
              # common build asset directories
              (lib.fileset.maybeMissing (crate + "/linker-scripts"))
              (lib.fileset.maybeMissing (crate + "/asm"))
              (lib.fileset.maybeMissing (crate + "/assets"))
            ];
          };

        kernel = craneLib.buildPackage (
          individualCrateArgs
          // {
            pname = "kernel";
            src = fileSetForCrate ./crates/kernel;
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
      in
      {
        packages = {
          inherit kernel;
          default = kernel;
          iso = iso;
        };
        apps.default = {
          type = "app";
          program = toString (
            pkgs.writeShellScript "run-qemu" ''
              set -e

              ISO=${iso}/kernel.iso

              OVMF_CODE=${pkgs.OVMF.fd}/FV/OVMF_CODE.fd
              OVMF_VARS=$(mktemp)

              cp ${pkgs.OVMF.fd}/FV/OVMF_VARS.fd $OVMF_VARS
              chmod +w $OVMF_VARS

              exec ${pkgs.qemu}/bin/qemu-system-x86_64 \
                -M q35 \
                -drive if=pflash,unit=0,format=raw,file=$OVMF_CODE,readonly=on \
                -drive if=pflash,unit=1,format=raw,file=$OVMF_VARS \
                -cdrom $ISO
            ''
          );
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
