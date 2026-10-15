# YARO

**Yet Another Rust OS**

YARO is a hobby operating system written in Rust, with the goal of supporting
x64, RISC-V, and ARM64.

## Current state

YARO is in early development. The kernel currently has:

- Limine boot initialization and bootable BIOS/UEFI ISO images.
- x86_64 GDT, TSS, and IDT initialization, CPU exception handlers, and a dedicated
  double-fault stack.
- UART 16550 serial output and kernel logging with an in-memory message history.
- Physical and virtual address types, page-table traversal, and address
  translation through Limine's higher-half direct map (HHDM).

The boot sequence initializes these subsystems, logs
`Kernel started successfully!` to the serial port, and halts the CPU. QEMU stays
open after initialization.

The kernel target is `x86_64-unknown-none`. Linker scripts for other architectures
are present, but their kernel support is incomplete.

## Build and run

Cargo builds the kernel, and [just](https://just.systems/) handles Limine setup,
ISO creation, and QEMU. Nix users can use the same commands inside the development
shell or build packages through Crane.

Run all commands from the repository root.

### Native setup

Install the following tools:

- [Rustup](https://rustup.rs/). The repository's `rust-toolchain.toml` selects
  the pinned nightly, components, and kernel target automatically.
- [just](https://just.systems/) 1.39 or newer.
- Git and a host C compiler (`cc`, or set `CC` to a compiler executable).
- `xorriso` for ISO creation.
- `qemu-system-x86_64` and OVMF/EDK II x86 firmware for UEFI emulation.
- Bash and the usual Unix command-line tools.

On Debian/Ubuntu, after installing rustup and just:

```bash
sudo apt install build-essential git xorriso qemu-system-x86 ovmf
```

Then build and boot YARO:

```bash
just run
```

This builds the debug kernel, sets up Limine, creates `target/kernel.iso`, and
starts QEMU. Limine is fetched at the commit pinned in `limine.rev`; its assets
and host installer are cached in `target/limine` for subsequent runs.

Individual steps are also available:

| Command | Result |
| --- | --- |
| `just setup` | Fetch Limine and compile its host installer |
| `just build` | Build `target/x86_64-unknown-none/debug/yaro_kernel` |
| `just iso` | Build the kernel and create `target/kernel.iso` |
| `just run` | Build the ISO and boot it in QEMU |

Use the release profile for an optimized kernel:

```bash
just profile=release build
just profile=release run
```

`CARGO_TARGET_DIR` overrides the location of build artifacts, the ISO, and the
Limine cache. To use an existing Limine installation, set `LIMINE_DIR` to its
asset directory and `LIMINE` to a matching installer executable.

### Nix setup

Enable Nix's `flakes` and `nix-command` features, then enter the development shell:

```bash
nix develop
just run
```

The shell supplies the pinned Rust toolchain, just, xorriso, QEMU, GDB, Limine,
and firmware paths. The repository also includes an `.envrc` for direnv.

To build and run directly:

```bash
nix run                       # Alias: nix run .#dev
nix run . -- -display none    # Forward extra QEMU arguments
```

Development builds use Cargo's local `target/` cache. For Nix package outputs,
Crane builds the release kernel and caches dependency artifacts:

| Command | Output |
| --- | --- |
| `nix build` | `result/bin/yaro_kernel` |
| `nix build .#iso` | `result/kernel.iso` |

The Nix ISO package uses the same `just package-iso` recipe as native builds,
with the kernel ELF supplied by Crane.

## Running and debugging

The QEMU launcher boots with UEFI, sends serial output to the terminal, and
opens a GDB server at `127.0.0.1:1234`. Extra arguments are forwarded to QEMU:

```bash
just run -display none    # Run headlessly with serial output in the terminal
just run -S              # Pause the CPU until a debugger resumes it
```

The launcher finds OVMF firmware in common Linux installation directories. If
it cannot find your firmware, set both paths explicitly:

```bash
export OVMF_CODE=/path/to/OVMF_CODE.fd
export OVMF_VARS=/path/to/OVMF_VARS.fd
just run
```

Use matching code and variable files from the same firmware package, such as
`OVMF_CODE_4M.fd` and `OVMF_VARS_4M.fd`. Each run uses a temporary writable copy
of the variable template. The Nix environment configures these paths automatically.
