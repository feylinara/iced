use crate::atlas::{self, allocator};

/// An allocated space in a texture atlas, usually part of an [`Entry`]
#[derive(Debug)]
pub enum Allocation {
    /// Alloacted space taking up part of a layer
    Partial {
        /// The layer space is allocated in
        layer: usize,
        /// Where the allocation is situated inside of the layer
        region: allocator::Region,
    },
    /// Allocated space taking up a full layer
    Full {
        /// The layer space is allocated in
        layer: usize,
    },
}

impl Allocation {
    /// Get the top-left corner of the allocation inside of the texture layer
    pub fn position(&self) -> (u32, u32) {
        match self {
            Allocation::Partial { region, .. } => region.position(),
            Allocation::Full { .. } => (0, 0),
        }
    }

    /// Get the size corner of the allocation
    pub fn size(&self) -> (u32, u32) {
        match self {
            Allocation::Partial { region, .. } => region.size(),
            Allocation::Full { .. } => (atlas::SIZE, atlas::SIZE),
        }
    }

    /// Get the texture layer in which the allocation is situated
    pub fn layer(&self) -> usize {
        match self {
            Allocation::Partial { layer, .. } => *layer,
            Allocation::Full { layer } => *layer,
        }
    }
}
