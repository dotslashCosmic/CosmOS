# CosmOS Build System
set shell := ["powershell.exe", "-c"]

# Default: build and run
default: build run-qemu

# Setup Rust + nightly toolchain, Chocolatey, NASM
setup:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 setup

# Build kernel, release
build:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build

# Build kernel, debug
build-debug:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build -Mode debug

# Build just the kernel ELF, no bootloader or disk image
build-kernel:
    @Write-Host "Building kernel ELF only..." -ForegroundColor Cyan
    $env:RUSTFLAGS = "-C link-arg=-Tkernel/linker.ld"
    cargo build --package cosmos --target x86_64-unknown-none --release
    $env:RUSTFLAGS = ""
    @Write-Host "Kernel ELF: target/x86_64-unknown-none/release/cosmos" -ForegroundColor Green

# Build just the kernel ELF, debug mode
build-kernel-debug:
    @Write-Host "Building kernel ELF (debug) only..." -ForegroundColor Cyan
    $env:RUSTFLAGS = "-C link-arg=-Tkernel/linker.ld"
    cargo build --package cosmos --target x86_64-unknown-none --debug
    $env:RUSTFLAGS = ""
    @Write-Host "Kernel ELF: target/x86_64-unknown-none/debug/cosmos" -ForegroundColor Green

# Clean and rebuild
rebuild: 
    cargo clean
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 clean
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build

# Run in QEMU
run-qemu: 
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 run-qemu

# Run in VirtualBox
run-vbox:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 run-vbox

# Create VirtualBox VDI disk
create-vdi:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 create-vdi

# Update and restart VirtualBox VM
update-vm:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 update-vm

# Clean build artifacts
clean:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 clean
