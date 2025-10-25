//! Page Table Management

use super::{PhysicalAddress, PhysicalFrame};
use super::memory_map::{MemoryMap, MemoryType};

/// Page table entry flags
const PAGE_PRESENT: u64 = 1 << 0;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_SIZE: u64 = 1 << 7; // 2MB pages

/// Page table addresses
const PML4_ADDRESS: usize = 0x70000;
const PDPT_ADDRESS: usize = 0x71000;
const PD_BASE_ADDRESS: usize = 0x72000;

/// Errors that can occur during paging operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingError {
    OutOfMemory,
    InvalidAddress,
    Corruption,
}

/// Currently mapped memory size
static MAPPED_MEMORY: spin::Mutex<usize> = spin::Mutex::new(0);

/// Initialize paging to map all available physical memory
pub fn init_full_memory_mapping(memory_map: &MemoryMap) -> Result<usize, PagingError> {
    // Detect how much memory the bootloader actually mapped by checking page tables
    let initial_mapped = detect_mapped_memory();
    
    // Get total usable memory from memory map
    let total_usable = memory_map.total_usable_memory() as usize;
    
    // Store the detected mapping
    *MAPPED_MEMORY.lock() = initial_mapped;
    
    // Calculate how much more we need to map
    let target_mapped = total_usable.min(4 * 1024 * 1024 * 1024); // TODO: Dynamically adjust
    
    if target_mapped > initial_mapped {
        // TODO: Implement dynamic page table expansion
        Ok(initial_mapped)
    } else {
        Ok(initial_mapped)
    }
}

/// Detect how much memory is currently mapped by examining page tables
fn detect_mapped_memory() -> usize {
    unsafe {
        let pml4_ptr = PML4_ADDRESS as *const u64;
        let pdpt_ptr = PDPT_ADDRESS as *const u64;
        
        // Check if PML4[0] is present
        if (*pml4_ptr & 1) == 0 {
            return 0;
        }
        
        // Count how many PDPT entries are present
        let mut pd_count = 0;
        for i in 0..512 {
            if (*pdpt_ptr.add(i) & 1) != 0 {
                pd_count = i + 1;
            } else {
                break; // Stop at first non-present entry
            }
        }
        
        if pd_count == 0 {
            return 0;
        }
        
        // Count entries in each PD
        let mut total_pages = 0;
        for pd_idx in 0..pd_count {
            let pd_ptr = (PD_BASE_ADDRESS + pd_idx * 0x1000) as *const u64;
            for entry_idx in 0..512 {
                if (*pd_ptr.add(entry_idx) & 1) != 0 {
                    total_pages += 1;
                } else {
                    break; // Stop at first non-present entry in this PD
                }
            }
        }
        
        // Each page is 2MB
        total_pages * 2 * 1024 * 1024
    }
}

/// Get the amount of memory currently mapped
pub fn get_mapped_memory() -> usize {
    *MAPPED_MEMORY.lock()
}
