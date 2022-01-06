//! Utilities to store images for caching.
//!
//! This is not a texture cache, but it can be used to build one
//!
//! # Examples
//!
//! ```
//! struct NullBackend {}
//!
//! impl Backend for NullBackend {
//!     type Texture = ();
//!
//!     fn texture(&self) -> &Self::Texture {
//!         &()
//!     }
//! }
//! impl NullBackend {
//!     fn grow(width: u32, height: u32, context: ()) {}
//!     fn upload(width: u32, height: u32, data: &[u8], entry: &Entry, context: ())
//! }
//! let atlas = Atlas::new(NullBackend);
//!
//! let image = [1, 2, 3, 4];
//! let width = 2;
//! let height = 2;
//!
//! let entry =
//!  atlas.entry_for(width, height, |backend, layers, amount| {
//!    backend.grow(layers, amount, ())
//!  })?;
//! atlas
//!  .backend_mut()
//!  .upload(width, height, &image, &entry, ());
//!
//! atlas.remove(&entry);
//! ```

pub mod entry;

mod allocation;
mod allocator;
mod layer;

use std::num::NonZeroU32;

pub use allocation::Allocation;
pub use entry::Entry;
pub use layer::Layer;

use allocator::Allocator;

/// The size of texture atlasses.
pub const SIZE: u32 = 2048;

/// A Backend interfacing between the image atlas and the storage, usually a GPU texture.
pub trait Backend: std::fmt::Debug {
    /// The type of the texture the renderer needs access to to display images.
    type Texture;

    /// The texture the renderer needs access to to display images.
    fn texture(&self) -> &Self::Texture;
}

/// A texture atlas as a store for caching images
#[derive(Debug)]
pub struct Atlas<B: Backend> {
    backend: B,
    layers: Vec<Layer>,
}

impl<B: Backend> Atlas<B> {
    /// Create a new atlas
    pub fn new(backend: B) -> Self {
        Atlas {
            backend,
            layers: vec![Layer::Empty],
        }
    }

    /// The texture the renderer needs access to to display images.
    pub fn view(&self) -> &B::Texture {
        &self.backend.texture()
    }

    /// The amount of layers that memory is allocated for (but not the amount of
    /// actually allocated layers)
    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    /// Allocate an [`Entry`] for an image with given width and height
    ///
    /// grow should increase the amount of memory available for the texture
    /// atlas, while preserving the already uploaded data. A list of layers is
    /// provided to avoid unecessary copying
    pub fn entry_for(
        &mut self,
        width: u32,
        height: u32,
        grow: impl FnOnce(&mut B, &[Layer], usize),
    ) -> Option<Entry> {
        let current_size = self.layers.len();
        let entry = self.allocate(width, height)?;

        // We grow the internal texture after allocating if necessary
        let new_layers = self.layers.len() - current_size;
        grow(&mut self.backend, &self.layers, new_layers);

        Some(entry)
    }

    /// Access the backend
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Access the backend mutably
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Allow the allocated memory for the entry to be reused
    pub fn remove(&mut self, entry: &Entry) {
        match entry {
            Entry::Contiguous(allocation) => {
                self.deallocate(allocation);
            }
            Entry::Fragmented { fragments, .. } => {
                for fragment in fragments {
                    self.deallocate(&fragment.allocation);
                }
            }
        }
    }

    fn allocate(&mut self, width: u32, height: u32) -> Option<Entry> {
        // Allocate one layer if texture fits perfectly
        if width == SIZE && height == SIZE {
            let mut empty_layers = self
                .layers
                .iter_mut()
                .enumerate()
                .filter(|(_, layer)| layer.is_empty());

            if let Some((i, layer)) = empty_layers.next() {
                *layer = Layer::Full;

                return Some(Entry::Contiguous(Allocation::Full { layer: i }));
            }

            self.layers.push(Layer::Full);

            return Some(Entry::Contiguous(Allocation::Full {
                layer: self.layers.len() - 1,
            }));
        }

        // Split big textures across multiple layers
        if width > SIZE || height > SIZE {
            let mut fragments = Vec::new();
            let mut y = 0;

            while y < height {
                let height = std::cmp::min(height - y, SIZE);
                let mut x = 0;

                while x < width {
                    let width = std::cmp::min(width - x, SIZE);

                    let allocation = self.allocate(width, height)?;

                    if let Entry::Contiguous(allocation) = allocation {
                        fragments.push(entry::Fragment {
                            position: (x, y),
                            allocation,
                        });
                    }

                    x += width;
                }

                y += height;
            }

            return Some(Entry::Fragmented {
                size: (width, height),
                fragments,
            });
        }

        // Try allocating on an existing layer
        for (i, layer) in self.layers.iter_mut().enumerate() {
            match layer {
                Layer::Empty => {
                    let mut allocator = Allocator::new(SIZE);

                    if let Some(region) = allocator.allocate(width, height) {
                        *layer = Layer::Busy(allocator);

                        return Some(Entry::Contiguous(Allocation::Partial {
                            region,
                            layer: i,
                        }));
                    }
                }
                Layer::Busy(allocator) => {
                    if let Some(region) = allocator.allocate(width, height) {
                        return Some(Entry::Contiguous(Allocation::Partial {
                            region,
                            layer: i,
                        }));
                    }
                }
                _ => {}
            }
        }

        // Create new layer with atlas allocator
        let mut allocator = Allocator::new(SIZE);

        if let Some(region) = allocator.allocate(width, height) {
            self.layers.push(Layer::Busy(allocator));

            return Some(Entry::Contiguous(Allocation::Partial {
                region,
                layer: self.layers.len() - 1,
            }));
        }

        // We ran out of memory (?)
        None
    }

    fn deallocate(&mut self, allocation: &Allocation) {
        match allocation {
            Allocation::Full { layer } => {
                self.layers[*layer] = Layer::Empty;
            }
            Allocation::Partial { layer, region } => {
                let layer = &mut self.layers[*layer];

                if let Layer::Busy(allocator) = layer {
                    allocator.deallocate(region);

                    if allocator.is_empty() {
                        *layer = Layer::Empty;
                    }
                }
            }
        }
    }
}
