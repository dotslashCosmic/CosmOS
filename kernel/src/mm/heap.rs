//! Kernel Heap Allocator

use super::frame_allocator::allocate_frame;
use super::PhysicalFrame;
use linked_list_allocator::LockedHeap;
use spin::Mutex;

/// Heap configuration constants
pub const HEAP_START: usize = 0x400000; // 4MB
pub const MIN_HEAP_SIZE: usize = 4 * 1024 * 1024; // 4MB minimum
pub const MAX_HEAP_SIZE: usize = 256 * 1024 * 1024; // 256MB maximum

/// Global allocator instance
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Heap initialization state
static HEAP_INITIALIZED: Mutex<bool> = Mutex::new(false);

/// Actual heap size (determined at runtime)
static HEAP_SIZE: Mutex<usize> = Mutex::new(0);

/// Errors that can occur during heap operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeapError {
    /// Heap is already initialized
    AlreadyInitialized,
    /// Failed to allocate physical frames for heap
    FrameAllocationFailed,
    /// Invalid heap configuration
    InvalidConfiguration,
    /// Heap corruption detected
    CorruptionDetected,
}

impl core::fmt::Display for HeapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HeapError::AlreadyInitialized => write!(f, "Heap already initialized"),
            HeapError::FrameAllocationFailed => write!(f, "Failed to allocate frames for heap"),
            HeapError::InvalidConfiguration => write!(f, "Invalid heap configuration"),
            HeapError::CorruptionDetected => write!(f, "Heap corruption detected"),
        }
    }
}

/// Initialize the kernel heap with dynamic sizing
pub fn init_heap(total_usable_memory: u64) -> Result<(), HeapError> {
    let mut initialized = HEAP_INITIALIZED.lock();
    if *initialized {
        return Err(HeapError::AlreadyInitialized);
    }
    
    // Validate heap configuration
    if HEAP_START % PhysicalFrame::SIZE as usize != 0 {
        return Err(HeapError::InvalidConfiguration);
    }
    
    // Calculate heap size dynamically    
    const LOW_MEMORY_RESERVED: usize = 0x100000;      // 1MB
    const KERNEL_RESERVED: usize = 0x200000;          // 2MB (0x200000-0x400000)
    const OVERHEAD_RESERVED: usize = 0x200000;        // 2MB for stacks/tables
    const TOTAL_RESERVED: usize = LOW_MEMORY_RESERVED + KERNEL_RESERVED + OVERHEAD_RESERVED;
    
    // Heap gets everything else that's mapped and usable
    let mapped_memory = super::paging::get_mapped_memory();
    
    // Calculate: mapped memory - heap start address = available for heap
    // (heap starts at 0x400000, so everything from there to end of mapped memory)
    let available_for_heap = mapped_memory.saturating_sub(HEAP_START);
    
    // Clamp to min/max bounds
    let final_heap_size = available_for_heap
        .max(MIN_HEAP_SIZE)
        .min(MAX_HEAP_SIZE);
    
    // Round down to frame boundary
    let final_heap_size = (final_heap_size / PhysicalFrame::SIZE as usize) 
        * PhysicalFrame::SIZE as usize;
    
    if final_heap_size < MIN_HEAP_SIZE {
        return Err(HeapError::InvalidConfiguration);
    }
    
    // Store the actual heap size
    *HEAP_SIZE.lock() = final_heap_size;

    // Initialize the heap allocator with dynamic size
    unsafe {
        ALLOCATOR.lock().init(HEAP_START as *mut u8, final_heap_size);
    }
    *initialized = true;
    Ok(())
}

/// Check if the heap is initialized
pub fn is_initialized() -> bool {
    *HEAP_INITIALIZED.lock()
}

/// Get heap statistics
pub fn heap_stats() -> HeapStats {
    let heap = ALLOCATOR.lock();
    let total_size = *HEAP_SIZE.lock();
    HeapStats {
        total_size,
        used_size: heap.used(),
        free_size: heap.free(),
        start_address: HEAP_START,
    }
}

/// Heap statistics
#[derive(Debug, Clone, Copy)]
pub struct HeapStats {
    pub total_size: usize,
    pub used_size: usize,
    pub free_size: usize,
    pub start_address: usize,
}

/// Poison memory with a pattern for security
pub fn poison_memory(ptr: *mut u8, size: usize) {
    unsafe {
        // Use a recognizable poison pattern
        const POISON_PATTERN: u8 = 0xDE;
        for i in 0..size {
            *ptr.add(i) = POISON_PATTERN;
        }
    }
}

/// Check if memory contains poison pattern
pub fn is_poisoned(ptr: *const u8, size: usize) -> bool {
    unsafe {
        const POISON_PATTERN: u8 = 0xDE;
        for i in 0..size {
            if *ptr.add(i) != POISON_PATTERN {
                return false;
            }
        }
        true
    }
}

/// Allocate memory with additional security features
pub fn secure_alloc(size: usize) -> Option<*mut u8> {
    if !is_initialized() {
        return None;
    }
    if size > 4096 {
        // TODO: Add guard pages for large allocations
    }
    
    // Use the global allocator
    use core::alloc::{GlobalAlloc, Layout};
    
    let layout = Layout::from_size_align(size, 8).ok()?;
    unsafe {
        let ptr = ALLOCATOR.alloc(layout);
        if !ptr.is_null() {
            // Clear allocated memory
            core::ptr::write_bytes(ptr, 0, size);
            Some(ptr)
        } else {
            None
        }
    }
}

/// Deallocate memory with security features
pub fn secure_dealloc(ptr: *mut u8, size: usize) {
    if ptr.is_null() || !is_initialized() {
        return;
    }
    
    // Poison the memory before deallocation
    poison_memory(ptr, size);
    
    // Deallocate using global allocator
    use core::alloc::{GlobalAlloc, Layout};
    
    if let Ok(layout) = Layout::from_size_align(size, 8) {
        unsafe {
            ALLOCATOR.dealloc(ptr, layout);
        }
    }
}
