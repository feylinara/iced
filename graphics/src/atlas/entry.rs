//! The full data needed to allocate and deallocate space in an [`Atlas`].

use crate::atlas;

/// The full data needed to allocate and deallocate space in an [`Atlas`].
///
/// This is needed to deallocate the image and should be kept around
#[derive(Debug)]
pub enum Entry {
    /// A single allocation containing all of the image.
    Contiguous(atlas::Allocation),
    /// Several allocations containing the image together.
    Fragmented {
        /// The size of the image.
        size: (u32, u32),
        /// The fragments conatining parts of the image.
        fragments: Vec<Fragment>,
    },
}

impl Entry {
    /// The size of the image.
    pub fn size(&self) -> (u32, u32) {
        match self {
            Entry::Contiguous(allocation) => allocation.size(),
            Entry::Fragmented { size, .. } => *size,
        }
    }
}

/// A allocation for part of the image
#[derive(Debug)]
pub struct Fragment {
    /// The position of the part of the image that space is allocated for inside
    /// of the image
    pub position: (u32, u32),
    /// The allocation containing the part of the image.
    pub allocation: atlas::Allocation,
}
