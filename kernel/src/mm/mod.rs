//! Memory Management Module

pub mod memory_map;
pub mod frame_allocator;
pub mod heap;

// Re-export core types
pub use memory_map::{MemoryMap, MemoryMapEntry, MemoryType, MemoryMapError};
pub use frame_allocator::{FrameAllocator, AllocationError};

/// Physical address type with alignment and arithmetic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalAddress(u64);

impl PhysicalAddress {
    /// Create a new physical address
    pub const fn new(addr: u64) -> Self {
        PhysicalAddress(addr)
    }
    
    /// Get the raw address value
    pub const fn as_u64(self) -> u64 {
        self.0
    }
    
    /// Check if the address is aligned to the given alignment
    pub const fn is_aligned(self, alignment: u64) -> bool {
        self.0 % alignment == 0
    }
    
    /// Align the address up to the given alignment
    pub const fn align_up(self, alignment: u64) -> Self {
        PhysicalAddress((self.0 + alignment - 1) & !(alignment - 1))
    }
    
    /// Align the address down to the given alignment
    pub const fn align_down(self, alignment: u64) -> Self {
        PhysicalAddress(self.0 & !(alignment - 1))
    }
}

impl core::ops::Add<u64> for PhysicalAddress {
    type Output = Self;
    
    fn add(self, rhs: u64) -> Self::Output {
        PhysicalAddress(self.0 + rhs)
    }
}

impl core::ops::Sub<u64> for PhysicalAddress {
    type Output = Self;
    
    fn sub(self, rhs: u64) -> Self::Output {
        PhysicalAddress(self.0 - rhs)
    }
}

impl core::ops::Sub<PhysicalAddress> for PhysicalAddress {
    type Output = u64;
    
    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        self.0 - rhs.0
    }
}

/// Physical frame representing a 4KB page of physical memory
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhysicalFrame {
    start_address: PhysicalAddress,
}

impl PhysicalFrame {
    /// Size of a physical frame, 4KB
    pub const SIZE: u64 = 4096;
    
    /// Create a frame containing the given address
    pub const fn containing_address(addr: PhysicalAddress) -> Self {
        PhysicalFrame {
            start_address: addr.align_down(Self::SIZE),
        }
    }
    
    /// Get the start address of this frame
    pub const fn start_address(self) -> PhysicalAddress {
        self.start_address
    }
    
    /// Get the end address of this frame, exclusive
    pub const fn end_address(self) -> PhysicalAddress {
        PhysicalAddress(self.start_address.0 + Self::SIZE)
    }
    
    /// Get the frame number (address / frame size)
    pub const fn number(self) -> u64 {
        self.start_address.0 / Self::SIZE
    }
    
    /// Create a frame from a frame number
    pub const fn from_number(number: u64) -> Self {
        PhysicalFrame {
            start_address: PhysicalAddress(number * Self::SIZE),
        }
    }
}

impl core::ops::Add<u64> for PhysicalFrame {
    type Output = Self;
    
    fn add(self, rhs: u64) -> Self::Output {
        PhysicalFrame::from_number(self.number() + rhs)
    }
}

impl core::ops::Sub<u64> for PhysicalFrame {
    type Output = Self;
    
    fn sub(self, rhs: u64) -> Self::Output {
        PhysicalFrame::from_number(self.number() - rhs)
    }
}

/// Range of physical frames
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicalFrameRange {
    start: PhysicalFrame,
    end: PhysicalFrame,
}

impl PhysicalFrameRange {
    /// Create a new frame range
    pub const fn new(start: PhysicalFrame, end: PhysicalFrame) -> Self {
        PhysicalFrameRange { start, end }
    }
    
    /// Get the start frame
    pub const fn start(self) -> PhysicalFrame {
        self.start
    }
    
    /// Get the end frame (exclusive)
    pub const fn end(self) -> PhysicalFrame {
        self.end
    }
    
    /// Check if the range is empty
    pub const fn is_empty(self) -> bool {
        self.start.number() >= self.end.number()
    }
    
    /// Get the number of frames in this range
    pub const fn len(self) -> u64 {
        if self.is_empty() {
            0
        } else {
            self.end.number() - self.start.number()
        }
    }
}

impl Iterator for PhysicalFrameRange {
    type Item = PhysicalFrame;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.start < self.end {
            let frame = self.start;
            self.start = self.start + 1;
            Some(frame)
        } else {
            None
        }
    }
}