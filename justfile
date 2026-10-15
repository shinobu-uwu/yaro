set positional-arguments := true
set script-interpreter := ["bash", "-euo", "pipefail"]

profile := "dev"
target := "x86_64-unknown-none"
target-dir := env("CARGO_TARGET_DIR", justfile_directory() / "target")
artifact-profile := if profile == "dev" { "debug" } else { profile }
kernel := target-dir / target / artifact-profile / "yaro_kernel"
iso-path := target-dir / "kernel.iso"
limine-dir := env("LIMINE_DIR", target-dir / "limine")
limine-installer := env("LIMINE", limine-dir / "limine")

# List available commands.
default:
    @{{ quote(just_executable()) }} --list

# Build the kernel ELF. Use `just profile=release build` for a release build.
build:
    cargo build --locked -p yaro_kernel --target {{ quote(target) }} --profile {{ quote(profile) }} --target-dir {{ quote(target-dir) }}

# Fetch the pinned Limine binary release and compile its host installer once.
[script]
setup:
    limine_dir={{ quote(limine-dir) }}
    installer={{ quote(limine-installer) }}
    if [[ -n "${LIMINE_DIR:-}" ]]; then
        if [[ ! -d "$limine_dir" || ! -x "$installer" ]]; then
            echo "LIMINE_DIR must contain Limine assets, and LIMINE must name its executable installer." >&2
            exit 1
        fi
        exit 0
    fi

    revision=$(tr -d '\n' < {{ quote(justfile_directory() / "limine.rev") }})
    if [[ -f "$limine_dir/.yaro-revision" && -x "$installer" ]] && \
        [[ $(cat "$limine_dir/.yaro-revision") == "$revision" ]]; then
        exit 0
    fi

    mkdir -p "$(dirname "$limine_dir")"
    work_dir=$(mktemp -d "$limine_dir.XXXXXX")
    trap 'rm -rf "$work_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    git -C "$work_dir" init --quiet
    git -C "$work_dir" fetch --depth=1 https://github.com/limine-bootloader/limine.git "$revision"
    git -C "$work_dir" checkout --quiet --detach "$revision"
    "${CC:-cc}" -O2 -std=c99 "$work_dir/limine.c" -o "$work_dir/limine"
    printf '%s\n' "$revision" > "$work_dir/.yaro-revision"
    rm -rf "$limine_dir"
    mv "$work_dir" "$limine_dir"

# Package an existing ELF; this recipe never builds or fetches dependencies.
[script]
package-iso $kernel $limine_dir $config $output $installer=limine-installer:
    mkdir -p "$(dirname "$output")"
    work_dir=$(mktemp -d "$(dirname "$output")/.iso.XXXXXX")
    trap 'rm -rf "$work_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    iso_root="$work_dir/iso_root"
    mkdir -p "$iso_root/boot/limine" "$iso_root/EFI/BOOT"
    cp "$kernel" "$iso_root/boot/kernel"
    cp "$config" "$iso_root/boot/limine/limine.conf"
    cp "$limine_dir/limine-bios.sys" "$limine_dir/limine-bios-cd.bin" \
        "$limine_dir/limine-uefi-cd.bin" "$iso_root/boot/limine/"
    cp "$limine_dir/BOOTX64.EFI" "$limine_dir/BOOTIA32.EFI" "$iso_root/EFI/BOOT/"
    xorriso -as mkisofs -b boot/limine/limine-bios-cd.bin \
        -no-emul-boot -boot-load-size 4 -boot-info-table \
        --efi-boot boot/limine/limine-uefi-cd.bin \
        -efi-boot-part --efi-boot-image --protective-msdos-label \
        "$iso_root" -o "$work_dir/kernel.iso"
    "$installer" bios-install "$work_dir/kernel.iso"
    mv "$work_dir/kernel.iso" "$output"

# Build a bootable BIOS/UEFI ISO at target/kernel.iso.
iso: build setup
    {{ quote(just_executable()) }} --justfile {{ quote(justfile()) }} package-iso {{ quote(kernel) }} {{ quote(limine-dir) }} {{ quote(justfile_directory() / "limine.conf") }} {{ quote(iso-path) }} {{ quote(limine-installer) }}

# Build and boot in QEMU with UEFI. Extra arguments are forwarded to QEMU.
[script]
run *args: iso
    code="${OVMF_CODE:-}"
    vars="${OVMF_VARS:-}"
    if [[ -z "$code" && -z "$vars" ]]; then
        for firmware_dir in /usr/share/OVMF /usr/share/edk2/ovmf /usr/share/edk2/x64 /usr/share/qemu; do
            for suffix in _4M ''; do
                if [[ -f "$firmware_dir/OVMF_CODE$suffix.fd" && -f "$firmware_dir/OVMF_VARS$suffix.fd" ]]; then
                    code="$firmware_dir/OVMF_CODE$suffix.fd"
                    vars="$firmware_dir/OVMF_VARS$suffix.fd"
                    break 2
                fi
            done
        done
    fi
    if [[ ! -f "$code" || ! -f "$vars" ]]; then
        echo "Set OVMF_CODE and OVMF_VARS to a matching pair of OVMF firmware files (see README.md)." >&2
        exit 1
    fi

    run_dir=$(mktemp -d {{ quote(target-dir / "qemu-dev.XXXXXX") }})
    trap 'rm -rf "$run_dir"' EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    cp "$vars" "$run_dir/OVMF_VARS.fd"
    chmod u+w "$run_dir/OVMF_VARS.fd"
    qemu-system-x86_64 \
        -M q35 \
        -gdb tcp:127.0.0.1:1234 \
        -serial stdio \
        -drive "if=pflash,unit=0,format=raw,file=$code,readonly=on" \
        -drive "if=pflash,unit=1,format=raw,file=$run_dir/OVMF_VARS.fd" \
        -cdrom {{ quote(iso-path) }} "$@"

# Format all Rust crates; `just fmt --check` checks formatting.
fmt *args:
    cargo fmt --all -- "$@"
