//! GpuRenderer: punto de entrada principal de cupcraft-render.
//! Inicializa wgpu en modo headless (sin ventana), expone métodos para:
//! - `apply_effect(rgba_in, effect, time, amount) -> Vec<u8>` (para preview y exportación)
//! - `blend_transition(frame_a, frame_b, kind, progress, w, h) -> Vec<u8>`

use wgpu::{
    util::{DeviceExt, TextureDataOrder},
    AddressMode, BindingResource, Buffer, BufferDescriptor, BufferUsages,
    Color, CommandEncoder, CommandEncoderDescriptor, Device, DeviceDescriptor, Extent3d,
    Features, FilterMode, Limits, LoadOp, MapMode, MemoryHints, Operations, Origin3d,
    PollType, PowerPreference, Queue, RenderPassColorAttachment, RenderPassDescriptor,
    RequestAdapterOptions, Sampler, SamplerDescriptor, StoreOp, TexelCopyBufferInfo,
    TexelCopyBufferLayout, TexelCopyTextureInfo, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages, COPY_BYTES_PER_ROW_ALIGNMENT,
};
use bytemuck::cast_slice;
use crate::{
    effect_map::EffectId,
    pipeline::{PostPipeline, TransitionPipeline},
    transition::TransitionKind,
    uniforms::{PostUniforms, TransitionUniforms},
};

const QUAD: &[[f32; 2]] = &[[-1.0, -1.0], [1.0, -1.0], [-1.0, 1.0], [1.0, 1.0]];

/// Contexto GPU headless. Se crea una vez y se reutiliza.
pub struct GpuRenderer {
    pub device: Device,
    pub queue: Queue,
    format: TextureFormat,
    sampler: Sampler,
}

impl GpuRenderer {
    /// Inicializa wgpu. Bloquea hasta que el adaptador y el dispositivo están listos.
    pub fn new() -> anyhow::Result<Self> {
        pollster::block_on(Self::new_async())
    }

    async fn new_async() -> anyhow::Result<Self> {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                ..Default::default()
            })
            .await?;
        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("cupcraft-render"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                experimental_features: Default::default(),
                memory_hints: MemoryHints::Performance,
                trace: Default::default(),
            })
            .await?;
        let format = TextureFormat::Rgba8UnormSrgb;
        let sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("main_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..Default::default()
        });
        Ok(Self {
            device,
            queue,
            format,
            sampler,
        })
    }

    /// Aplica un efecto de post-proceso a un fotograma RGBA en CPU.
    /// Devuelve el fotograma procesado como Vec<u8> RGBA.
    pub fn apply_effect(
        &self,
        rgba_in: &[u8],
        width: u32,
        height: u32,
        effect: &EffectId,
        time: f32,
        amount: f32,
    ) -> Vec<u8> {
        let pipeline = PostPipeline::new(&self.device, self.format, effect.clone());
        let tex = self.upload_texture(rgba_in, width, height);
        let uniforms = PostUniforms {
            time,
            amount,
            res_x: width as f32,
            res_y: height as f32,
        };
        let uni_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("post_uniforms"),
            contents: cast_slice(&[uniforms]),
            usage: BufferUsages::UNIFORM,
        });
        let quad_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad"),
            contents: cast_slice(QUAD),
            usage: BufferUsages::VERTEX,
        });
        let tex_view = tex.create_view(&Default::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("post_bg"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uni_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&tex_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let (out_tex, out_buf) = self.create_output(width, height);
        let out_view = out_tex.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("post_enc") });
        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("post_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&pipeline.pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            rp.set_vertex_buffer(0, quad_buf.slice(..));
            rp.draw(0..4, 0..1);
        }
        self.copy_texture_to_buffer(&mut encoder, &out_tex, &out_buf, width, height);
        self.queue.submit(std::iter::once(encoder.finish()));
        self.read_buffer(&out_buf, width, height)
    }

    /// Mezcla dos fotogramas con una transición.
    pub fn blend_transition(
        &self,
        frame_a: &[u8],
        frame_b: &[u8],
        width: u32,
        height: u32,
        kind: TransitionKind,
        progress: f32,
    ) -> Vec<u8> {
        let pipeline = TransitionPipeline::new(&self.device, self.format);
        let tex_a = self.upload_texture(frame_a, width, height);
        let tex_b = self.upload_texture(frame_b, width, height);
        let uniforms = TransitionUniforms {
            progress,
            transition_type: kind.as_u32(),
            width: width as f32,
            height: height as f32,
        };
        let uni_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("transition_uniforms"),
            contents: cast_slice(&[uniforms]),
            usage: BufferUsages::UNIFORM,
        });
        let quad_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("quad"),
            contents: cast_slice(QUAD),
            usage: BufferUsages::VERTEX,
        });
        let view_a = tex_a.create_view(&Default::default());
        let view_b = tex_b.create_view(&Default::default());
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transition_bg"),
            layout: &pipeline.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uni_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(&view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        let (out_tex, out_buf) = self.create_output(width, height);
        let out_view = out_tex.create_view(&Default::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor { label: Some("tr_enc") });
        {
            let mut rp = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("tr_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &out_view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&pipeline.pipeline);
            rp.set_bind_group(0, &bind_group, &[]);
            rp.set_vertex_buffer(0, quad_buf.slice(..));
            rp.draw(0..4, 0..1);
        }
        self.copy_texture_to_buffer(&mut encoder, &out_tex, &out_buf, width, height);
        self.queue.submit(std::iter::once(encoder.finish()));
        self.read_buffer(&out_buf, width, height)
    }

    // --- helpers privados ---

    fn upload_texture(&self, rgba: &[u8], width: u32, height: u32) -> Texture {
        self.device.create_texture_with_data(
            &self.queue,
            &TextureDescriptor {
                label: Some("upload_tex"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: self.format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            },
            TextureDataOrder::LayerMajor,
            rgba,
        )
    }

    fn create_output(&self, width: u32, height: u32) -> (Texture, Buffer) {
        let tex = self.device.create_texture(&TextureDescriptor {
            label: Some("out_tex"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let aligned_stride = align_up(width * 4, COPY_BYTES_PER_ROW_ALIGNMENT);
        let buf = self.device.create_buffer(&BufferDescriptor {
            label: Some("out_buf"),
            size: (aligned_stride * height) as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        (tex, buf)
    }

    fn copy_texture_to_buffer(
        &self,
        encoder: &mut CommandEncoder,
        tex: &Texture,
        buf: &Buffer,
        width: u32,
        height: u32,
    ) {
        let aligned_stride = align_up(width * 4, COPY_BYTES_PER_ROW_ALIGNMENT);
        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: tex,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: buf,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(aligned_stride),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn read_buffer(&self, buf: &Buffer, width: u32, height: u32) -> Vec<u8> {
        let aligned_stride = align_up(width * 4, COPY_BYTES_PER_ROW_ALIGNMENT);
        let slice = buf.slice(..);
        slice.map_async(MapMode::Read, |_| {});
        let _ = self.device.poll(PollType::wait_indefinitely());
        let raw = slice.get_mapped_range().expect("Failed to get mapped range");
        let mut out = Vec::with_capacity((width * height * 4) as usize);
        for row in 0..height {
            let start = (row * aligned_stride) as usize;
            let end = start + (width * 4) as usize;
            out.extend_from_slice(&raw[start..end]);
        }
        drop(raw);
        buf.unmap();
        out
    }
}

fn align_up(n: u32, align: u32) -> u32 {
    (n + align - 1) & !(align - 1)
}
