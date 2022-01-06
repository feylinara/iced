use crate::atlas::Allocator;

/// A layer of memory allocated for use in an [`Atlas`].
#[derive(Debug)]
pub enum Layer {
    /// A layer with no space allocated.
    Empty,
    /// A layer with some space allocated. Owns an [`Allocator`] that can
    /// allocate or deallocate space in the layer.
    Busy(Allocator),
    /// A layer with all its space allocated.
    Full,
}

impl Layer {
    /// True if the layer has no space allocated.
    pub fn is_empty(&self) -> bool {
        match self {
            Layer::Empty => true,
            _ => false,
        }
    }
}
