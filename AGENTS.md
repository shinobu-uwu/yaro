# Repository Guidelines

## Project Structure & Module Organization

This Cargo workspace contains a Rust 2024 freestanding kernel using `#![no_std]` and `#![no_main]`.

- `crates/kernel/src/main.rs`: kernel entry point, Limine requests, framebuffer demonstration, and panic/halt handlers. Add kernel modules under `crates/kernel/src/`.
- `crates/kernel/build.rs`: selects `crates/kernel/linker-scripts/linker-{arch}.ld` for the target architecture.
- `justfile`: native Cargo builds, pinned Limine setup, shared ISO packaging, and QEMU launcher.
- `rust-toolchain.toml`: pinned nightly and target, shared by rustup and Nix's rust-overlay.
- `limine.rev`: pinned Limine binary release commit, shared by native setup and Nix.
- `flake.nix` and `flake.lock`: Nix development environment, Crane kernel builds, and ISO packaging through just.
- `limine.conf`: boot menu and kernel path configuration.
- `.cargo/config.toml`: kernel target and compiler flags.

The configured Nix build targets `x86_64-unknown-none`; additional linker scripts do not imply complete platform support. There are currently no dedicated test or asset directories.

## Build, Test, and Development Commands

Run commands from the repository root. Native dependencies are documented in README.md:

- `just setup`: fetch pinned Limine assets and compile their host installer into `target/limine`.
- `just build`: build the debug kernel with Cargo; `just profile=release build` selects release.
- `just iso`: build a bootable image at `target/kernel.iso`.
- `just run`: build the ISO and boot QEMU with UEFI; extra arguments are forwarded to QEMU.
- `just fmt --check`: check Rust formatting; `just fmt` applies it.

With Nix's `flakes` and `nix-command` features enabled:

- `nix develop`: enter the Rust development environment; `.envrc` also supports direnv.
- `nix build`: build the kernel ELF at `result/bin/yaro_kernel`.
- `nix build .#iso`: create the bootable image at `result/kernel.iso`.
- `nix run`: build the debug kernel with Cargo's local `target/` cache, package `target/kernel.iso`, and boot QEMU with UEFI firmware. Run from the repository root; use `nix run . -- ...` for extra QEMU flags. `nix run .#dev` is an alias.
- `cargo fmt --all -- --check`: check Rust formatting inside the development shell; use `cargo fmt --all` to apply formatting.

## Coding Style & Naming Conventions

Use rustfmt defaults with four-space Rust indentation. Use `snake_case` for functions and modules, `PascalCase` for types, and `SCREAMING_SNAKE_CASE` for constants and statics. Match existing two-space indentation in Nix files.

Keep kernel code compatible with `core`; avoid `std` dependencies. Explain safety assumptions around raw pointers and inline assembly. Preserve Limine request attributes and linker sections when changing boot initialization.

## Testing Guidelines

No automated test framework or coverage threshold is configured; Nix builds disable checks with `doCheck = false`. Validate kernel changes with `just iso` and `just run`, or `nix build .#iso` and `nix run`. Check serial output and the framebuffer. Record observed boot behavior and reproduction steps in the pull request.

## Commit & Pull Request Guidelines

History contains only `Initial commit`, so no established commit convention exists. Use concise, imperative subjects, such as `Add framebuffer bounds checks`, and keep commits focused.

Describe the change, motivation, and validation commands in pull requests. Link relevant issues and include screenshots for framebuffer changes. Call out target, linker, or boot configuration changes explicitly.
