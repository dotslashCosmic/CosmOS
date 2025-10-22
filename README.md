# CosmOS

CosmOS (Custom Open-Source Modular Operating System) is a fully custom kernel and bootloader written in Rust and Assembly. It is designed with memory safety and exploit resistance as core principles.

## Architecture

- Target: x86_64 bare-metal (`x86_64-unknown-none`)
- Language: Rust (nightly toolchain, edition 2021)
- Boot: Custom two-stage bootloader with BIOS/UEFI support
- Environment: `#![no_std]` with selective `alloc` usage
- Security & memory safety:
  - Hardware-enforced privilege separation (Ring 0 / Ring 3)
  - Rust ownership model to reduce buffer overflows and use-after-free
- Minimal runtime dependencies to keep the attack surface small

## Prerequisites

- Rust (nightly)
- NASM assembler
- QEMU or VirtualBox (for testing)
- On Windows, the setup may use winget (PowerShell) for installing tools

## Quick start

Run the included `just` tasks to set up, build and run:

```bash
# one-line setup, build, and run (PowerShell may be required for setup on Windows)
just
# or run steps individually:
just setup
just build
just run-qemu   # QEMU
# or
just run-vbox   # VirtualBox
```

## Boot process

1. Stage 1: 512-byte MBR bootloader (sector 0)  
2. Stage 2: Extended bootloader (sectors 1–64, ~32 KB)  
3. Kernel: Flat binary loaded at sector 66+

The bootloader creates a 64 MB disk image with the kernel embedded, avoiding a separate filesystem for initial boot.

## Development

Dependencies are compiled with `default-features = false` for `no_std` compatibility:
- `x86_64` — hardware abstractions
- `spin` — synchronization primitives
- `linked_list_allocator` — heap management

## License

This project is licensed under the GNU GPL v3.
