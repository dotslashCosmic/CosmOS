# CosmOS Development Script
param(
    [Parameter(Position=0)]
    [ValidateSet("setup", "build", "run-qemu", "run-vbox", "create-vdi", "update-vm", "clean", "help")]
    [string]$Command = "help",
    
    [ValidateSet("debug", "release")]
    [string]$Mode = "release",
    
    [switch]$Force
)

$ErrorActionPreference = "Stop"

# Color functions for better output
function Write-Header($text) { Write-Host $text -ForegroundColor Cyan }
function Write-Success($text) { Write-Host $text -ForegroundColor Green }
function Write-Info($text) { Write-Host $text -ForegroundColor Yellow }
function Write-Error($text) { Write-Host $text -ForegroundColor Red }
function Write-Detail($text) { Write-Host $text -ForegroundColor Gray }

# Common paths
$script:BootDir = "boot\src"
$script:TargetDir = "target\x86_64-unknown-none\$Mode"
$script:VBoxPath = "C:\Program Files\Oracle\VirtualBox\VBoxManage.exe"
$script:UUID = "{01f11b20-4db0-4c0b-89c0-c288455c3e73}"

function Show-Help {
    Write-Header "CosmOS Development Script"
    Write-Host ""
    Write-Host "Usage: .\cosmos.ps1 <command> [options]"
    Write-Host ""
    Write-Host "Commands:"
    Write-Host "  setup       Install development dependencies"
    Write-Host "  build       Build the kernel and bootloader"
    Write-Host "  run-qemu    Build and run in QEMU"
    Write-Host "  run-vbox    Build and run in VirtualBox"
    Write-Host "  create-vdi  Convert bootimage to VirtualBox VDI"
    Write-Host "  update-vm   Quick update and restart VirtualBox VM"
    Write-Host "  clean       Clean build artifacts"
    Write-Host "  help        Show this help"
    Write-Host ""
    Write-Host "Options:"
    Write-Host "  -Mode <debug|release>  Build mode (default: release)"
    Write-Host "  -Force                 Force reinstall dependencies"
    Write-Host ""
    Write-Host "Examples:"
    Write-Host "  .\cosmos.ps1 setup"
    Write-Host "  .\cosmos.ps1 build -Mode debug"
    Write-Host "  .\cosmos.ps1 run-qemu"
}

function Install-Dependencies {
    Write-Header "CosmOS Environment Setup"
    Write-Host ""
    
    Write-Host ""
    Write-Success "Checking dependencies..."
    
    $hasWinget = Get-Command winget -ErrorAction SilentlyContinue
    $needsManual = $false
    
    # Check Rust
    Write-Info "Checking Rust..."
    if (Get-Command rustc -ErrorAction SilentlyContinue) {
        Write-Success "Rust found"
        Write-Info "Setting up Rust components..."
        rustup toolchain install nightly
        rustup default nightly
        rustup component add rust-src llvm-tools-preview
        rustup target add x86_64-unknown-none
        Write-Success "Rust components configured"
    } else {
        if ($hasWinget) {
            Write-Info "Installing Rust via winget..."
            winget install Rustlang.Rustup
            Write-Success "Rust installed - restart PowerShell and run setup again"
            $needsManual = $true
        } else {
            Write-Error "Rust not found!"
            Write-Info "Install Rust from: https://rustup.rs/"
            Write-Info "Then run this setup again."
            $needsManual = $true
        }
    }
    
    # Check NASM
    Write-Info "Checking NASM..."
    if ((Get-Command nasm -ErrorAction SilentlyContinue) -and (-not $Force)) {
        Write-Success "NASM already installed"
    } else {
        if ($hasWinget) {
            Write-Info "Installing NASM via winget..."
            winget install nasm.nasm
            # Refresh PATH
            $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
            Write-Success "NASM installed"
        } else {
            Write-Error "NASM not found!"
            Write-Info "Install options:"
            Write-Info "1. Download from: https://www.nasm.us/"
            Write-Info "2. Update Windows to get winget"
            $needsManual = $true
        }
    }
    
    # Check QEMU (optional)
    Write-Info "Checking QEMU..."
    $qemuExists = (Get-Command qemu-system-x86_64 -ErrorAction SilentlyContinue) -or (Test-Path "C:\Program Files\qemu\qemu-system-x86_64.exe")
    if ($qemuExists -and (-not $Force)) {
        Write-Success "QEMU already installed"
    } else {
        if ($hasWinget) {
            Write-Info "Installing QEMU via winget..."
            winget install SoftwareFreedomConservancy.QEMU
            Write-Success "QEMU installed"
        } else {
            Write-Info "QEMU not found (optional for testing)"
            Write-Info "Install from: https://www.qemu.org/"
        }
    }
    
    Write-Host ""
    if ($needsAdmin) {
        Write-Error "Some dependencies are missing!"
        Write-Info "Please install the missing components and run setup again."
    } else {
        Write-Success "Setup Complete!"
        Write-Host ""
        Write-Info "Next steps:"
        Write-Host "- Build: .\cosmos.ps1 build"
        Write-Host "- Run: .\cosmos.ps1 run-qemu"
    }
}

function Find-Tool($toolName, $commonPaths = @()) {
    # Refresh environment variables first
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
    
    $tool = Get-Command $toolName -ErrorAction SilentlyContinue
    if ($tool) {
        return $tool.Source
    }
    
    # Try common paths
    foreach ($path in $commonPaths) {
        if (Test-Path $path) {
            return $path
        }
    }
    
    return $null
}

function Build-CosmOS {
    Write-Header "=== Building CosmosBootloader ==="
    Write-Host ""
    
    # File paths
    $stage1Asm = "$script:BootDir\stage1.asm"
    $stage2Asm = "$script:BootDir\stage2.asm"
    $stage1Bin = "$script:TargetDir\stage1.bin"
    $stage2Bin = "$script:TargetDir\stage2.bin"
    $kernelElf = "$script:TargetDir\cosmos"
    $bootImage = "$script:TargetDir\bootimage-cosmos.bin"
    
    # Create target directory
    New-Item -ItemType Directory -Force -Path $script:TargetDir | Out-Null
    
    # Find NASM
    $nasmPaths = @(
        "C:\ProgramData\chocolatey\bin\nasm.exe",
        "C:\Program Files\NASM\nasm.exe"
    )
    $nasm = Find-Tool "nasm" $nasmPaths
    
    if (-not $nasm) {
        Write-Error "NASM assembler not found!"
        Write-Info "Run setup to install dependencies: .\cosmos.ps1 setup"
        Write-Info "Then restart PowerShell or refresh environment"
        exit 1
    }
    Write-Info "Using NASM: $nasm"
    
    # Build Stage 1
    Write-Success "[1/5] Assembling Stage 1 bootloader..."
    & $nasm -f bin $stage1Asm -o $stage1Bin
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Stage 1 assembly failed"
        exit 1
    }
    Write-Detail "  [OK] Stage 1: $stage1Bin (512 bytes)"
    
    # Build Stage 2
    Write-Success "[2/5] Assembling Stage 2 bootloader..."
    & $nasm -f bin $stage2Asm -o $stage2Bin
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Stage 2 assembly failed"
        exit 1
    }
    Write-Detail "  [OK] Stage 2: $stage2Bin (32KB)"
    
    # Build kernel
    Write-Success "[3/5] Building kernel..."
    $env:RUSTFLAGS = "-C link-arg=-Tkernel/linker.ld"
    cargo build --package cosmos --target x86_64-unknown-none --$Mode
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Kernel build failed"
        exit 1
    }
    $env:RUSTFLAGS = ""
    
    if (-not (Test-Path $kernelElf)) {
        Write-Error "Kernel binary not found at $kernelElf"
        exit 1
    }
    $kernelSize = (Get-Item $kernelElf).Length
    Write-Detail "  [OK] Kernel: $kernelElf ($kernelSize bytes)"
    
    # Create flat binary
    Write-Success "[4/5] Creating flat kernel binary..."
    
    # Get entry point address
    $objdump = Get-ChildItem -Path "$env:USERPROFILE\.rustup\toolchains" -Recurse -Filter "llvm-objdump.exe" | Select-Object -First 1
    if ($objdump) {
        $entryInfo = & $objdump.FullName -f $kernelElf | Select-String "start address"
        Write-Info "  Entry point: $entryInfo"
    }
    
    # Convert ELF to flat binary
    $kernelBin = "$script:TargetDir\cosmos.bin"
    $objcopy = Get-Command llvm-objcopy -ErrorAction SilentlyContinue
    if (-not $objcopy) {
        $objcopy = Get-ChildItem -Path "$env:USERPROFILE\.rustup\toolchains" -Recurse -Filter "llvm-objcopy.exe" | Select-Object -First 1
        if ($objcopy) {
            $objcopy = $objcopy.FullName
        } else {
            Write-Error "llvm-objcopy not found!"
            exit 1
        }
    } else {
        $objcopy = $objcopy.Source
    }
    
    & $objcopy -O binary $kernelElf $kernelBin
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to create flat binary"
        exit 1
    }
    $kernelBinSize = (Get-Item $kernelBin).Length
    Write-Detail "  [OK] Flat binary: $kernelBin ($kernelBinSize bytes)"
    
    # Create bootable image
    Write-Success "[5/5] Creating bootable disk image..."
    
    # Create 64MB disk image
    $diskSize = 64MB
    $bytes = New-Object byte[] $diskSize
    
    # Write Stage 1 (MBR) - sector 0
    $stage1Bytes = [System.IO.File]::ReadAllBytes($stage1Bin)
    [Array]::Copy($stage1Bytes, 0, $bytes, 0, $stage1Bytes.Length)
    Write-Detail "  [OK] Stage 1 written to sector 0 (512 bytes)"
    
    # Write Stage 2 - sectors 1-64 (starts at byte 512)
    $stage2Bytes = [System.IO.File]::ReadAllBytes($stage2Bin)
    [Array]::Copy($stage2Bytes, 0, $bytes, 512, $stage2Bytes.Length)
    Write-Detail "  [OK] Stage 2 written to sectors 1-64 (32KB)"
    
    # Write kernel - starts at sector 66 (byte 33280)
    $kernelBytes = [System.IO.File]::ReadAllBytes($kernelBin)
    $kernelOffset = 66 * 512  # Sector 66
    [Array]::Copy($kernelBytes, 0, $bytes, $kernelOffset, $kernelBytes.Length)
    Write-Detail "  [OK] Kernel written starting at sector 66 ($kernelBinSize bytes)"
    
    # Write the complete image
    [System.IO.File]::WriteAllBytes($bootImage, $bytes)
    
    Write-Host ""
    Write-Success "=== Build Complete ==="
    Write-Info "Boot image: $bootImage"
    Write-Info "Image size: $diskSize bytes (64MB)"
    Write-Host ""
    Write-Info "Disk layout:"
    Write-Detail "  Sector 0:      Stage 1 (MBR)"
    Write-Detail "  Sectors 1-64:  Stage 2 (32KB)"
    Write-Detail "  Sector 66+:    Kernel ($kernelBinSize bytes)"
}

function Run-QEMU {
    Build-CosmOS
    
    Write-Host ""
    Write-Success "Starting QEMU..."
    
    $bootImage = "$script:TargetDir\bootimage-cosmos.bin"
    $qemu = Find-Tool "qemu-system-x86_64" @("C:\Program Files\qemu\qemu-system-x86_64.exe")
    
    if (-not $qemu) {
        Write-Error "QEMU not found! Run setup first: .\cosmos.ps1 setup"
        exit 1
    }
    
    & $qemu -drive format=raw,file=$bootImage -serial stdio
}

function Create-VDI {
    $bootImage = "$script:TargetDir\bootimage-cosmos.bin"
    $vdiPath = "$script:TargetDir\cosmos.vdi"
    
    # Check if bootimage exists
    if (-not (Test-Path $bootImage)) {
        Write-Error "Boot image not found at $bootImage"
        Write-Info "Please build the kernel first with: .\cosmos.ps1 build"
        exit 1
    }
    
    Write-Success "Creating VirtualBox disk from bootimage..."
    $bootImageSize = (Get-Item $bootImage).Length
    Write-Info "Boot image: $bootImageSize bytes"
    
    Write-Info "Converting to VDI format..."
    
    # Remove old VDI if it exists
    if (Test-Path $vdiPath) {
        Remove-Item $vdiPath -Force
    }
    
    # Convert bootimage directly to VDI
    & $script:VBoxPath convertfromraw $bootImage $vdiPath --format VDI
    
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Failed to convert to VDI"
        exit 1
    }
    
    # Set UUID
    Write-Info "Setting UUID..."
    & $script:VBoxPath internalcommands sethduuid $vdiPath $script:UUID
    
    Write-Success "VirtualBox disk created!"
    Write-Info "Location: $vdiPath"
}

function Run-VirtualBox {
    Build-CosmOS
    Create-VDI
    
    Write-Success "Starting VirtualBox VM..."
    & $script:VBoxPath startvm CosmOS-Dev
}

function Update-VM {
    $ErrorActionPreference = "Continue"
    
    $vdiPath = "$script:TargetDir\cosmos.vdi"
    
    Write-Info "Stopping VM (if running)..."
    & $script:VBoxPath controlvm CosmOS-Dev poweroff 2>&1 | Out-Null
    Start-Sleep -Seconds 2
    
    Write-Info "Creating VirtualBox disk..."
    Create-VDI
    
    if (-not (Test-Path $vdiPath)) {
        Write-Error "VDI creation failed"
        exit 1
    }
    
    Write-Info "Attaching disk..."
    $vdiPathFull = (Resolve-Path $vdiPath).Path
    & $script:VBoxPath storageattach CosmOS-Dev --storagectl "SATA" --port 0 --device 0 --type hdd --medium none 2>&1 | Out-Null
    & $script:VBoxPath storageattach CosmOS-Dev --storagectl "SATA" --port 0 --device 0 --type hdd --medium $vdiPathFull
    
    Write-Success "Starting VM..."
    & $script:VBoxPath startvm CosmOS-Dev
}

function Clean-Build {
    Write-Info "Cleaning build artifacts..."
    
    # Run cargo clean
    cargo clean
    Write-Success "Cargo clean completed"
    
    # Clean additional files
    $filesToClean = @("serial.log", "VBox.log")
    foreach ($file in $filesToClean) {
        if (Test-Path $file) {
            Remove-Item $file -Force
            Write-Detail "Removed $file"
        }
    }
    
    Write-Success "All build artifacts cleaned"
}

# Main command dispatcher
switch ($Command) {
    "setup" { Install-Dependencies }
    "build" { Build-CosmOS }
    "run-qemu" { Run-QEMU }
    "run-vbox" { Run-VirtualBox }
    "create-vdi" { Create-VDI }
    "update-vm" { Update-VM }
    "clean" { Clean-Build }
    "help" { Show-Help }
    default { Show-Help }

}
