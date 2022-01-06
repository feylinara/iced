use iced_graphics::atlas::{Allocation, Backend, Entry, Layer, SIZE};

use std::num::NonZeroU32;

#[derive(Debug)]
pub struct WgpuBackend {
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
}

impl WgpuBackend {
    pub fn new(device: &wgpu::Device) -> Self {
        let extent = wgpu::Extent3d {
            width: SIZE,
            height: SIZE,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced_wgpu::image texture atlas"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        Self {
            texture,
            texture_view,
        }
    }

    fn upload_allocation(
        &mut self,
        buffer: &wgpu::Buffer,
        image_width: u32,
        image_height: u32,
        padding: u32,
        offset: usize,
        allocation: &Allocation,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        let (x, y) = allocation.position();
        let (width, height) = allocation.size();
        let layer = allocation.layer();

        let extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer,
                layout: wgpu::ImageDataLayout {
                    offset: offset as u64,
                    bytes_per_row: NonZeroU32::new(4 * image_width + padding),
                    rows_per_image: NonZeroU32::new(image_height),
                },
            },
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x,
                    y,
                    z: layer as u32,
                },
                aspect: wgpu::TextureAspect::default(),
            },
            extent,
        );
    }

    pub fn grow(
        &mut self,
        layers: &[Layer],
        amount: usize,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        if amount == 0 {
            return;
        }

        let new_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("iced_wgpu::image texture atlas"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: layers.len() as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
        });

        let amount_to_copy = layers.len() - amount;

        for (i, layer) in layers.iter().take(amount_to_copy).enumerate() {
            if layer.is_empty() {
                continue;
            }

            encoder.copy_texture_to_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::ImageCopyTexture {
                    texture: &new_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: i as u32,
                    },
                    aspect: wgpu::TextureAspect::default(),
                },
                wgpu::Extent3d {
                    width: SIZE,
                    height: SIZE,
                    depth_or_array_layers: 1,
                },
            );
        }

        self.texture = new_texture;
        self.texture_view =
            self.texture.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                ..Default::default()
            });
    }

    pub fn upload(
        &mut self,
        image_width: u32,
        image_height: u32,
        data: &[u8],
        entry: &Entry,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        use wgpu::util::DeviceExt;

        // It is a webgpu requirement that:
        //   BufferCopyView.layout.bytes_per_row % wgpu::COPY_BYTES_PER_ROW_ALIGNMENT == 0
        // So we calculate padded_width by rounding width up to the next
        // multiple of wgpu::COPY_BYTES_PER_ROW_ALIGNMENT.
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padding = (align - (4 * image_width) % align) % align;
        let padded_width = (4 * image_width + padding) as usize;
        let padded_data_size = padded_width * image_height as usize;

        let mut padded_data = vec![0; padded_data_size];

        for row in 0..image_height as usize {
            let offset = row * padded_width;

            padded_data[offset..offset + 4 * image_width as usize]
                .copy_from_slice(
                    &data[row * 4 * image_width as usize
                        ..(row + 1) * 4 * image_width as usize],
                )
        }

        let buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("iced_wgpu::image staging buffer"),
                contents: &padded_data,
                usage: wgpu::BufferUsages::COPY_SRC,
            });

        match &entry {
            Entry::Contiguous(allocation) => {
                self.upload_allocation(
                    &buffer,
                    image_width,
                    image_height,
                    padding,
                    0,
                    &allocation,
                    encoder,
                );
            }
            Entry::Fragmented { fragments, .. } => {
                for fragment in fragments {
                    let (x, y) = fragment.position;
                    let offset = (y * padded_width as u32 + 4 * x) as usize;

                    self.upload_allocation(
                        &buffer,
                        image_width,
                        image_height,
                        padding,
                        offset,
                        &fragment.allocation,
                        encoder,
                    );
                }
            }
        }
    }
}

impl Backend for WgpuBackend {
    type Texture = wgpu::TextureView;

    fn texture(&self) -> &Self::Texture {
        &self.texture_view
    }
}
