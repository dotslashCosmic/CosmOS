//! Memory Map Parsing

use super::{PhysicalAddress, PhysicalFrame, PhysicalFrameRange};

/// E820 memory map entry types
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Usable RAM
    Usable = 1,
    /// Reserved memory
    Reserved = 2,
    /// ACPI reclaimable memory
    AcpiReclaimable = 3,
    /// ACPI NVS memory
    AcpiNvs = 4,
    /// Bad memory
    BadMemory = 5,
}

impl MemoryType {
    /// Create a MemoryType from raw u32 value
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            1 => Some(MemoryType::Usable),
            2 => Some(MemoryType::Reserved),
            3 => Some(MemoryType::AcpiReclaimable),
            4 => Some(MemoryType::AcpiNvs),
            5 => Some(MemoryType::BadMemory),
            _ => None,
        }
    }
    
    /// Check if this memory type is usable for allocation
    pub fn is_usable(self) -> bool {
        matches!(self, MemoryType::Usable)
    }
}

/// A single memory map entry from the bootloader, 24 bytes total
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    /// Base address of the memory region, 8 bytes
    pub base_addr: u64,
    /// Length of the memory region in bytes, 8 bytes
    pub length: u64,
    /// Type of memory region, 4 bytes
    pub entry_type: u32,
    /// Extended attributes, 4 bytes, usually 1 for valid entries
    pub attributes: u32,
}

impl MemoryMapEntry {
    /// Get the memory type for this entry
    pub fn memory_type(&self) -> Option<MemoryType> {
        MemoryType::from_u32(self.entry_type)
    }
    
    /// Get the start address of this memory region
    pub fn start_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.base_addr)
    }
    
    /// Get the end address of this memory region, exclusive
    pub fn end_address(&self) -> PhysicalAddress {
        PhysicalAddress::new(self.base_addr + self.length)
    }
    
    /// Get the range of frames covered by this memory region
    pub fn frame_range(&self) -> PhysicalFrameRange {
        let start_frame = PhysicalFrame::containing_address(self.start_address());
        let end_frame = PhysicalFrame::containing_address(self.end_address() - 1) + 1;
        PhysicalFrameRange::new(start_frame, end_frame)
    }
    
    /// Check if this entry is valid and usable
    pub fn is_usable(&self) -> bool {
        self.attributes == 1 && 
        self.length > 0 && 
        self.memory_type().map_or(false, |t| t.is_usable())
    }
    
    /// Check if entry represents system/hardware reserved memory
    pub fn is_system_reserved(&self) -> bool {
        // Check for common system reserved regions
        let start = self.base_addr;
        let _end = self.base_addr + self.length;
        
        // BIOS/VGA regions
        if start < 0x100000 {
            return true;
        }
        
        // Check memory type
        match self.memory_type() {
            Some(MemoryType::Reserved) | Some(MemoryType::BadMemory) => true,
            _ => false,
        }
    }
    
    /// Check if this entry can be reclaimed later
    pub fn is_reclaimable(&self) -> bool {
        matches!(self.memory_type(), Some(MemoryType::AcpiReclaimable))
    }
    
    /// Get a human-readable description of this memory region
    pub fn description(&self) -> &'static str {
        match self.memory_type() {
            Some(MemoryType::Usable) => "Usable RAM",
            Some(MemoryType::Reserved) => "Reserved",
            Some(MemoryType::AcpiReclaimable) => "ACPI Reclaimable",
            Some(MemoryType::AcpiNvs) => "ACPI NVS",
            Some(MemoryType::BadMemory) => "Bad Memory",
            None => "Unknown",
        }
    }
}

/// Memory map provided by the bootloader
pub struct MemoryMap {
    entries: &'static [MemoryMapEntry],
    usable_memory: u64,
}

impl MemoryMap {
    /// Fixed location where bootloader stores memory map
    const MEMORY_MAP_LOCATION: usize = 0x9000;
    
    /// Create a fallback memory map when bootloader data is unavailable
    pub fn create_fallback() -> Self {
        // Create a static fallback memory map with reasonable defaults
        static FALLBACK_ENTRIES: [MemoryMapEntry; 2] = [
            // Conventional memory: 0 - 640KB
            MemoryMapEntry {
                base_addr: 0x0,
                length: 0x9FC00, // ~640KB
                entry_type: 1,   // Usable
                attributes: 1,
            },
            // Extended memory: 1MB - 128MB
            MemoryMapEntry {
                base_addr: 0x100000,  // 1MB
                length: 0x7F00000,    // 127MB
                entry_type: 1,        // Usable
                attributes: 1,
            },
        ];
        
        MemoryMap {
            entries: &FALLBACK_ENTRIES,
            usable_memory: 0x9FC00 + 0x7F00000, // ~128MB
        }
    }
    
    /// Parse memory map from bootloader data
    pub fn from_bootloader() -> Result<Self, MemoryMapError> {
        unsafe {
            // Bootloader stores 32-bit entry count, then enters
            let entry_count_ptr = Self::MEMORY_MAP_LOCATION as *const u32;
            let raw_entry_count = *entry_count_ptr;
            
            // Check if location contains reasonable data
            if raw_entry_count == 0 || raw_entry_count == 0xFFFFFFFF {
                return Err(MemoryMapError::NoMemoryMap);
            }
            
            // Convert to usize and validate
            let entry_count = raw_entry_count as usize;
            if entry_count > 64 {
                return Err(MemoryMapError::InvalidMemoryMap);
            }
            
            // Memory map entries start after the count, bootloader uses 4 byte alignment
            let entries_ptr = (Self::MEMORY_MAP_LOCATION + 4) as *const MemoryMapEntry;
            let entries = core::slice::from_raw_parts(entries_ptr, entry_count);
            
            // Validate entries and calculate total usable memory
            let mut usable_memory = 0;
            let mut highest_ram_addr = 0;
            let mut valid_entries = 0;
            
            for entry in entries.iter() {
                // Basic validation
                if entry.length == 0 {
                    continue; // Skip zero-length entries
                }
                
                // Check for address overflow
                if entry.base_addr.checked_add(entry.length).is_none() {
                    continue; // Skip entries that would overflow
                }
                
                // Check for reasonable base address
                if entry.base_addr < 0x1000 && entry.base_addr != 0 {
                    continue; // Skip suspicious low addresses except 0
                }
                
                // Check memory type is reasonable
                let mem_type = entry.memory_type();
                if mem_type.is_none() {
                    // Allow unknown types but don't count as usable
                    valid_entries += 1;
                    continue;
                }
                
                valid_entries += 1;
                
                // Track highest reclaimable RAM address
                if entry.is_usable() || entry.is_reclaimable() {
                    let end_addr = entry.base_addr + entry.length;
                    if end_addr > highest_ram_addr && end_addr < 0x100000000 {
                        highest_ram_addr = end_addr;
                    }
                }
                
                if entry.is_usable() {
                    usable_memory += entry.length;
                }
            }
            
            if valid_entries == 0 {
                return Err(MemoryMapError::InvalidMemoryMap);
            }
            
            // Estimate from highest RAM address
            if usable_memory < 16 * 1024 * 1024 || highest_ram_addr > usable_memory * 2 {
                if highest_ram_addr > 0 {
                    // Use highest RAM address as the total physical memory
                    usable_memory = (highest_ram_addr * 3) / 4;
                }
                // Ensure minimum of 128MB
                if usable_memory < 128 * 1024 * 1024 {
                    usable_memory = 128 * 1024 * 1024;
                }
            }
            
            let memory_map = MemoryMap {
                entries,
                usable_memory,
            };
            
            // Output debug information
            memory_map.debug_print();
            
            Ok(memory_map)
        }
    }
    
    /// Get total usable memory in bytes
    pub fn total_usable_memory(&self) -> u64 {
        self.usable_memory
    }
    
    /// Get total physical RAM in bytes
    pub fn total_physical_memory(&self) -> u64 {
        let mut total = 0u64;
        for entry in self.entries.iter() {
            // Count all memory types except hardware-mapped regions above 4GB, TODO: Dynamic sizing
            if entry.base_addr < 0x100000000 {
                total += entry.length;
            }
        }
        total
    }
    
    /// Get all memory map entries
    pub fn entries(&self) -> &[MemoryMapEntry] {
        self.entries
    }
    
    /// Iterator over usable memory regions
    pub fn usable_regions(&self) -> impl Iterator<Item = &MemoryMapEntry> {
        self.entries.iter().filter(|entry| entry.is_usable())
    }
    
    /// Iterator over usable frame ranges
    pub fn usable_frame_ranges(&self) -> impl Iterator<Item = PhysicalFrameRange> + '_ {
        self.usable_regions().map(|entry| entry.frame_range())
    }
    
    /// Find the largest usable memory region
    pub fn largest_usable_region(&self) -> Option<&MemoryMapEntry> {
        self.usable_regions().max_by_key(|entry| entry.length)
    }
    
    /// Validate memory map consistency
    pub fn validate(&self) -> Result<(), MemoryMapError> {
        // Simple validation without Vec - check for basic consistency
        for entry in self.entries {
            // Check for address overflow
            if entry.base_addr.checked_add(entry.length).is_none() {
                return Err(MemoryMapError::InvalidMemoryMap);
            }
            
            // Check for valid memory type
            if entry.memory_type().is_none() {
                return Err(MemoryMapError::InvalidMemoryMap);
            }
        }
        
        Ok(())
    }
    
    /// Print debug information about the memory map
    pub fn debug_print(&self) {
        // This would use serial output in a real implementation
        // For now, we'll just validate the structure
        let _ = self.validate();
        
        // Count different memory types
        let mut _usable_count = 0;
        let mut _reserved_count = 0;
        let mut _acpi_count = 0;
        let mut _other_count = 0;
        
        for entry in self.entries {
            match entry.memory_type() {
                Some(MemoryType::Usable) => _usable_count += 1,
                Some(MemoryType::Reserved) => _reserved_count += 1,
                Some(MemoryType::AcpiReclaimable) | Some(MemoryType::AcpiNvs) => _acpi_count += 1,
                _ => _other_count += 1,
            }
        }
        // TODO: Output physical memory details to serial
    }
    
    /// Get memory statistics
    pub fn stats(&self) -> MemoryMapStats {
        let mut stats = MemoryMapStats::default();
        
        for entry in self.entries {
            match entry.memory_type() {
                Some(MemoryType::Usable) => {
                    stats.usable_regions += 1;
                    stats.usable_memory += entry.length;
                }
                Some(MemoryType::Reserved) => {
                    stats.reserved_regions += 1;
                    stats.reserved_memory += entry.length;
                }
                Some(MemoryType::AcpiReclaimable) => {
                    stats.acpi_regions += 1;
                    stats.acpi_memory += entry.length;
                }
                Some(MemoryType::AcpiNvs) => {
                    stats.acpi_regions += 1;
                    stats.acpi_memory += entry.length;
                }
                Some(MemoryType::BadMemory) => {
                    stats.bad_regions += 1;
                    stats.bad_memory += entry.length;
                }
                None => {
                    stats.unknown_regions += 1;
                    stats.unknown_memory += entry.length;
                }
            }
        }
        stats
    }
}

/// Errors that can occur during memory map parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryMapError {
    /// No memory map found at expected location
    NoMemoryMap,
    /// Memory map data is invalid or corrupted
    InvalidMemoryMap,
    /// Insufficient memory detected, < 16MB
    InsufficientMemory,
}

impl core::fmt::Display for MemoryMapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MemoryMapError::NoMemoryMap => write!(f, "No memory map found"),
            MemoryMapError::InvalidMemoryMap => write!(f, "Invalid memory map data"),
            MemoryMapError::InsufficientMemory => write!(f, "Insufficient memory detected"),
        }
    }
}

/// Memory map stats
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryMapStats {
    pub usable_regions: u32,
    pub usable_memory: u64,
    pub reserved_regions: u32,
    pub reserved_memory: u64,
    pub acpi_regions: u32,
    pub acpi_memory: u64,
    pub bad_regions: u32,
    pub bad_memory: u64,
    pub unknown_regions: u32,
    pub unknown_memory: u64,
}
