# CosmOS
Custom Open-Source Modular Operating System is a fully custom kernel and bootloader, written in Rust and Assembly,, designed with memory safety and exploit resistance as core principles.
CosmOS implements a hybrid kernel architecture targeting x86_64 bare-metal systems, and VMs.

## Architecture

- **Target**: x86_64 bare-metal (`x86_64-unknown-none`)
- **Language**: Rust (nightly toolchain, edition 2021)
- **Boot**: Custom two-stage bootloader with BIOS/UEFI support
- **Environment**: `#![no_std]` with selective `alloc` usage
- **Security**: Hardware-enforced privilege separation (Ring 0/Ring 3)
- **Memory Safety**: Rust ownership system prevents buffer overflows and use-after-free
- **Privilege Separation**: Kernel (Ring 0) and userspace (Ring 3) isolation
- **Minimal Attack Surface**: `no_std` environment with carefully selected dependencies
- **Hardware Enforcement**: x86_64 protection rings and paging

## Prerequisites (installed during just setup)

- Windows 10/11 (winget)
- Rust nightly toolchain
- NASM assembler
- QEMU/VirtualBox (optional, for testing)

## Quick Start

Just type `just` for a one-line setup, build, and execution!

```bash
# Setup (PowerShell required for dependency installation)
just
# or
just setup
just build
just run-qemu # Qemu
# or
just run-vbox # VirtualBox
```

## Boot Process

1. **Stage 1**: 512-byte MBR bootloader (sector 0)
2. **Stage 2**: Extended bootloader (sectors 1-64, 32KB)
3. **Kernel**: Flat binary loaded at sector 66+

The bootloader creates a 64MB disk image with the kernel embedded directly, eliminating the need for a separate filesystem during boot.

## Development

### Dependencies
All dependencies use `default-features = false` for `no_std` compatibility:
- `x86_64` - Hardware abstractions
- `spin` - Synchronization primitives
- `linked_list_allocator` - Heap management

## License

This project is licensed under the GNU GPL 3.0 License.
