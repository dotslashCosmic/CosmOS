//! UEFI Boot Services

use super::EFI_STATUS;

/// Locate Handle search types
pub const BY_REGISTER_NOTIFY: u32 = 0;
pub const BY_PROTOCOL: u32 = 1;
pub const ALL_HANDLES: u32 = 2;
