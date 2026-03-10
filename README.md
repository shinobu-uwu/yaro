# Nix Limine Rust Template

A [Nix](https://nixos.org/)-based template for writing a kernel in Rust, bootable via [Limine](https://limine-bootloader.org/).
Inspired by the [official C template](https://codeberg.org/Limine/limine-c-template).

This repository is simply a starting point, and it does not aim to be any kind of framework to hold your hand and guide you
through the intricacies of OSDev. You **should** change it to fit the needs of your project.

## Requirements

- [Nix](https://nixos.org/download.html) with the `flakes` and `nix-command` features enabled.
- (Optional) [direnv](https://direnv.net/) for automatic `nix develop` shell activation.

## Usage

### Building

To build the raw kernel ELF executable:

```bash
nix build
# The compiled kernel will be available at ./result/bin/kernel
```

To build a bootable ISO image containing the kernel and Limine:

```bash
nix build .#iso
# The compiled ISO will be available at ./result/kernel.iso
```

### Running in QEMU

You can easily test the kernel in QEMU. The default `nix run` command automatically builds the ISO and boots it using QEMU with UEFI support (via OVMF).

```bash
nix run
```

### Development Environment

You can drop into a Nix development shell that contains the Rust toolchain (including `rust-analyzer` and `rust-src`), allowing you to use your standard IDE integrations seamlessly:

```bash
nix develop
```

## Project Structure

This template is structured to separate the bootable ISO generation from the Rust crate building:

- `flake.nix`: The main Nix configuration file that glues everything together (toolchains, crane builds, Limine, xorriso).
- `kernel/`: The directory containing the Rust kernel codebase (`Cargo.toml` and `src/`).
- `limine.conf`: The configuration file for the Limine bootloader, copied to the ISO during the `kernel-iso` derivation build phase.

This project makes use of [cargo workspaces](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html), so it is recommended you split your
project into small different crates.

