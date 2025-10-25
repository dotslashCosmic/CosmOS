# CosmOS Build System
set shell := ["powershell.exe", "-c"]

# Default: build and run in QEMU (BIOS)
default: run-qemu

# Setup development environment
setup:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 setup

# Build BIOS bootloader and kernel
build:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build

# Build in debug mode
build-debug:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build -Mode debug

# Build UEFI bootloader
build-uefi:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build-uefi

# Build UEFI bootloader in debug mode
build-uefi-debug:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 build-uefi -Mode debug

# Run in QEMU (BIOS mode)
run-qemu: 
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 run-qemu

# Run in QEMU (UEFI mode)
run-uefi-qemu:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 run-uefi-qemu

# Run in VirtualBox (BIOS mode)
run-vbox:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 run-vbox

# Create UEFI disk image
create-uefi-image:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 create-uefi-image

# Create VirtualBox VDI
create-vdi:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 create-vdi

# Update and restart VM
update-vm:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 update-vm

# Clean build artifacts
clean:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 clean

# Clean and rebuild
rebuild: clean build

# Build release artifacts (BIOS image, UEFI bootloader, kernel binary, and ELF)
release:
    powershell -ExecutionPolicy Bypass -File cosmos.ps1 release
