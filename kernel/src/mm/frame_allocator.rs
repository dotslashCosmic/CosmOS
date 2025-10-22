//! Physical Frame Allocator

use super::{PhysicalAddress, PhysicalFrame, MemoryMap};
use spin::Mutex;

/// Errors that can occur during frame allocation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationError {
    /// No more physical memory available
    OutOfMemory,
    /// Invalid frame address provided
    InvalidFrame,
    /// Frame is already allocated
    FrameAlreadyAllocated,
    /// Frame is not currently allocated
    FrameNotAllocated,
}

impl core::fmt::Display for AllocationError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AllocationError::OutOfMemory => write!(f, "Out of physical memory"),
            AllocationError::InvalidFrame => write!(f, "Invalid frame address"),
            AllocationError::FrameAlreadyAllocated => write!(f, "Frame already allocated"),
            AllocationError::FrameNotAllocated => write!(f, "Frame not allocated"),
        }
    }
}

/// Simple bitmap-based frame allocator
pub struct FrameAllocator {
    memory_map: MemoryMap,
    next_free_frame: PhysicalFrame,
    allocated_frames: u64,
    total_frames: u64,
}

impl FrameAllocator {
    /// Create a new frame allocator from a memory map
    pub fn new(memory_map: MemoryMap) -> Self {
        // Find the first usable frame after kernel space, assume kernel ends at 4MB
        let kernel_end = PhysicalAddress::new(4 * 1024 * 1024);
        let next_free_frame = PhysicalFrame::containing_address(kernel_end);
        
        // Calculate total available frames
        let total_frames = memory_map.total_usable_memory() / PhysicalFrame::SIZE;
        
        FrameAllocator {
            memory_map,
            next_free_frame,
            allocated_frames: 0,
            total_frames,
        }
    }
    
    /// Allocate a single physical frame
    pub fn allocate_frame(&mut self) -> Result<PhysicalFrame, AllocationError> {
        // TODO: Add more robust bitmap
        if self.allocated_frames >= self.total_frames {
            return Err(AllocationError::OutOfMemory);
        }
        
        // Find next available frame in usable regions
        for region in self.memory_map.usable_frame_ranges() {
            if self.next_free_frame >= region.start() && self.next_free_frame < region.end() {
                let frame = self.next_free_frame;
                self.next_free_frame = self.next_free_frame + 1;
                self.allocated_frames += 1;
                
                // Clear the frame for security
                self.clear_frame(frame);
                
                return Ok(frame);
            }
            
            // If current frame is before this region, jump to region start
            if self.next_free_frame < region.start() {
                self.next_free_frame = region.start();
                let frame = self.next_free_frame;
                self.next_free_frame = self.next_free_frame + 1;
                self.allocated_frames += 1;
                
                // Clear the frame for security
                self.clear_frame(frame);
                
                return Ok(frame);
            }
        }
        
        Err(AllocationError::OutOfMemory)
    }
    
    /// Deallocate a physical frame
    pub fn deallocate_frame(&mut self, frame: PhysicalFrame) -> Result<(), AllocationError> {
        // Verify frame is in a usable region
        let mut found_in_region = false;
        for region in self.memory_map.usable_frame_ranges() {
            if frame >= region.start() && frame < region.end() {
                found_in_region = true;
                break;
            }
        }
        
        if !found_in_region {
            return Err(AllocationError::InvalidFrame);
        }
        
        // Clear the frame for security
        self.clear_frame(frame);
        
        // Update allocation count
        if self.allocated_frames > 0 {
            self.allocated_frames -= 1;
        }
        
        // Reset next_free_frame if this frame is earlier
        if frame < self.next_free_frame {
            self.next_free_frame = frame;
        }
        Ok(())
    }
    
    /// Clear a frame's contents for security
    fn clear_frame(&self, frame: PhysicalFrame) {
        unsafe {
            let frame_ptr = frame.start_address().as_u64() as *mut u64;
            let frame_size_u64 = PhysicalFrame::SIZE / 8;
            
            for i in 0..frame_size_u64 {
                *frame_ptr.add(i as usize) = 0;
            }
        }
    }
    
    /// Get allocation statistics
    pub fn stats(&self) -> FrameAllocatorStats {
        FrameAllocatorStats {
            total_frames: self.total_frames,
            allocated_frames: self.allocated_frames,
            free_frames: self.total_frames - self.allocated_frames,
            total_memory: self.memory_map.total_usable_memory(),
            allocated_memory: self.allocated_frames * PhysicalFrame::SIZE,
        }
    }
}

/// Frame allocator statistics
#[derive(Debug, Clone, Copy)]
pub struct FrameAllocatorStats {
    pub total_frames: u64,
    pub allocated_frames: u64,
    pub free_frames: u64,
    pub total_memory: u64,
    pub allocated_memory: u64,
}

/// Global frame allocator instance
static FRAME_ALLOCATOR: Mutex<Option<FrameAllocator>> = Mutex::new(None);

/// Initialize the global frame allocator
pub fn init_frame_allocator(memory_map: MemoryMap) -> Result<(), AllocationError> {
    let mut allocator = FRAME_ALLOCATOR.lock();
    *allocator = Some(FrameAllocator::new(memory_map));
    Ok(())
}

/// Allocate a frame
pub fn allocate_frame() -> Result<PhysicalFrame, AllocationError> {
    let mut allocator = FRAME_ALLOCATOR.lock();
    match allocator.as_mut() {
        Some(alloc) => alloc.allocate_frame(),
        None => Err(AllocationError::OutOfMemory),
    }
}

/// Deallocate a frame
pub fn deallocate_frame(frame: PhysicalFrame) -> Result<(), AllocationError> {
    let mut allocator = FRAME_ALLOCATOR.lock();
    match allocator.as_mut() {
        Some(alloc) => alloc.deallocate_frame(frame),
        None => Err(AllocationError::InvalidFrame),
    }
}

/// Get frame allocator statistics
pub fn get_stats() -> Option<FrameAllocatorStats> {
    let allocator = FRAME_ALLOCATOR.lock();
    allocator.as_ref().map(|alloc| alloc.stats())
}