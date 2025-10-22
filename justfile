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
