// Build script to create a bootable disk image
use std::process::Command;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=boot/src/boot.asm");
}
