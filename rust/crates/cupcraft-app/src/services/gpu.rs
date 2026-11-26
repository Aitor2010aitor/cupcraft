//! Compositing en GPU (Fase B) — ejecución de los shaders del compositor por
//! cómputo con `wgpu` headless (sin ventana ni superficie).
//!
//! Dos pasadas corren en la tarjeta:
//!
//! * **Texturas GL** (`vignette`, `grain`, `posterize`, `rgb-split`): las cuatro
//!   texturas puras que en la web son shaders GLSL `f32`. Por eso la salida en
//!   GPU es *más* fiel al original que la aproximación `f64` en CPU.
//! * **Cadena de color** (`run_color`): el bucle por píxel de `rgba_to_pixmap`
//!   (invertir → brillo → contraste → gris → saturación → sepia → opacidad),
//!   más la viñeta del panel Ajustar y el premultiplicado. Corre en **cada**
//!   fotograma, así que es el mayor ahorro de tiempo. El giro de tono
//!   (`hue-rotate`, que usa la conversión HSV de egui en espacio lineal) y el
//!   chroma key conservan la ruta CPU exacta: el llamante no manda a la GPU los
//!   fotogramas que los necesitan.
//!
//! Los búferes se reservan por llamada según el tamaño real del fotograma (la
//! cadena de color corre a la resolución del medio, no del lienzo, y varía por
//! clip), así un único contexto sirve para todas las pasadas y resoluciones.
//!
//! Degradación elegante: si no hay adaptador GPU disponible (equipos sin
//! aceleración, controladores antiguos), `GpuCtx::new` devuelve `None` y el
//! compositor sigue usando la reserva en CPU (`rayon`). Nunca es un botón muerto.

use std::cell::RefCell;

/// Parámetros de una pasada de textura (`uniform`, 32 bytes, alineado a 16).
/// `mode`: 0 viñeta · 1 grano · 2 posterizar · 3 rgb-split.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct TexParams {
    w: u32,
    h: u32,
    mode: u32,
    _pad0: u32,
    time: f32,
    amount: f32,
    _pad1: f32,
    _pad2: f32,
}

/// Parámetros de la pasada de color (`uniform`, 32 bytes). `n`: número de
/// eslabones de la cadena · `vig`: intensidad de la viñeta del panel Ajustar.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ColParams {
    w: u32,
    h: u32,
    n: u32,
    _pad0: u32,
    vig: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
}

/// Búfer reutilizable entre fotogramas: se mantiene residente en la GPU y solo
/// se recrea cuando hace falta más capacidad (Paso B3). Evita reservar/liberar
/// en cada llamada, que es el coste dominante por fotograma.
#[derive(Default)]
struct PoolBuf {
    buf: Option<wgpu::Buffer>,
    cap: u64,
}

impl PoolBuf {
    /// Garantiza un búfer de al menos `need` bytes con `usage`. Crece pero nunca
    /// encoge: así clips de distinto tamaño reutilizan el mayor visto. Devuelve
    /// una referencia al búfer residente.
    fn ensure(
        &mut self,
        device: &wgpu::Device,
        need: u64,
        usage: wgpu::BufferUsages,
        label: &'static str,
    ) -> &wgpu::Buffer {
        if self.buf.is_none() || self.cap < need {
            self.cap = need.max(self.cap);
            self.buf = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: self.cap,
                usage,
                mapped_at_creation: false,
            }));
        }
        self.buf.as_ref().unwrap()
    }
}

/// Búferes residentes compartidos por las dos pasadas. `uniform` es fijo (32 B,
/// mismo tamaño en textura y color); los de datos crecen según el fotograma.
#[derive(Default)]
struct Pool {
    uniform: PoolBuf,
    src: PoolBuf,
    dst: PoolBuf,
    readback: PoolBuf,
    chain: PoolBuf,
}

/// Contexto GPU persistente de una exportación. Reutiliza dispositivo, cola, las
/// dos pipelines y los búferes de datos (Paso B3): quedan residentes en la GPU y
/// solo se recrean si un clip mayor necesita más capacidad.
pub struct GpuCtx {
    device: wgpu::Device,
    queue: wgpu::Queue,
    tex_pipeline: wgpu::ComputePipeline,
    tex_layout: wgpu::BindGroupLayout,
    col_pipeline: wgpu::ComputePipeline,
    col_layout: wgpu::BindGroupLayout,
    pool: RefCell<Pool>,
}

const TEX_SHADER: &str = r#"
struct P { w:u32, h:u32, mode:u32, pad0:u32, time:f32, amount:f32, pad1:f32, pad2:f32 };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var<storage, read> src: array<u32>;
@group(0) @binding(2) var<storage, read_write> dst: array<u32>;

fn unpack(v:u32) -> vec4<f32> {
    return vec4<f32>(
        f32(v & 0xffu),
        f32((v >> 8u) & 0xffu),
        f32((v >> 16u) & 0xffu),
        f32((v >> 24u) & 0xffu)
    ) / 255.0;
}
fn pack(c:vec4<f32>) -> u32 {
    let r = u32(clamp(round(c.x * 255.0), 0.0, 255.0));
    let g = u32(clamp(round(c.y * 255.0), 0.0, 255.0));
    let b = u32(clamp(round(c.z * 255.0), 0.0, 255.0));
    let a = u32(clamp(round(c.w * 255.0), 0.0, 255.0));
    return r | (g << 8u) | (b << 16u) | (a << 24u);
}
fn hash1(n:f32) -> f32 { return fract(sin(n) * 43758.5453); }
fn hash2(x:f32, y:f32) -> f32 { return fract(sin(x * 12.9898 + y * 78.233) * 43758.5453); }
fn sstep(e0:f32, e1:f32, x:f32) -> f32 {
    let t = clamp((x - e0) / (e1 - e0), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= p.w || y >= p.h) { return; }
    let idx = y * p.w + x;
    var c = unpack(src[idx]);
    let amt = p.amount;

    if (p.mode == 0u) {
        // vignette: rgb * mix(0.45, 1.0, smoothstep(0.78, 0.32, d*(0.9 + amt*0.5)))
        let ux = (f32(x) + 0.5) / f32(p.w);
        let uy = (f32(y) + 0.5) / f32(p.h);
        let d = sqrt((ux - 0.5) * (ux - 0.5) + (uy - 0.5) * (uy - 0.5));
        let v = sstep(0.78, 0.32, d * (0.9 + amt * 0.5));
        let m = mix(0.45, 1.0, v);
        c = vec4<f32>(c.xyz * m, c.w);
    } else if (p.mode == 1u) {
        // grain: rgb + (hash(uv*res + time) - 0.5) * 0.22 * amt
        let n = hash2(f32(x) + 0.5 + p.time, f32(y) + 0.5 + p.time) - 0.5;
        c = vec4<f32>(c.xyz + n * 0.22 * amt, c.w);
    } else if (p.mode == 2u) {
        // posterize: floor(c*steps + 0.5) / steps, steps = max(2, 12 - amt*9)
        let steps = max(2.0, 12.0 - amt * 9.0);
        c = vec4<f32>(floor(c.xyz * steps + 0.5) / steps, c.w);
    } else {
        // rgb-split: desplaza R/B por bandas de fila; G y alfa sin tocar.
        let uy = (f32(y) + 0.5) / f32(p.h);
        let band = floor(uy * 42.0);
        let shift = (hash1(band + floor(p.time * 8.0)) - 0.5) * amt * 0.05;
        let dxf = shift * f32(p.w);
        let xr = u32(clamp(round(f32(x) + dxf), 0.0, f32(p.w - 1u)));
        let xb = u32(clamp(round(f32(x) - dxf), 0.0, f32(p.w - 1u)));
        let r = unpack(src[y * p.w + xr]).x;
        let b = unpack(src[y * p.w + xb]).z;
        c = vec4<f32>(r, c.y, b, c.w);
    }

    dst[idx] = pack(c);
}
"#;

// Pasada de color: réplica en `f32` del bucle por píxel de `rgba_to_pixmap`.
// `chain[2*i]`   = (invert, brightness, contrast, grayscale)
// `chain[2*i+1]` = (saturate, sepia, opacity[sin uso aquí], _)
// La cadena opera sobre RGB con alfa fijo a 255 y su alfa de salida se descarta
// (el fotograma conserva su propio alfa), igual que la CPU. Cada eslabón
// cuantiza a u8 antes del siguiente, como `apply_fx`.
const COL_SHADER: &str = r#"
struct P { w:u32, h:u32, n:u32, pad0:u32, vig:f32, pad1:f32, pad2:f32, pad3:f32 };
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var<storage, read> src: array<u32>;
@group(0) @binding(2) var<storage, read_write> dst: array<u32>;
@group(0) @binding(3) var<storage, read> chain: array<vec4<f32>>;

fn unpack(v:u32) -> vec4<f32> {
    return vec4<f32>(
        f32(v & 0xffu),
        f32((v >> 8u) & 0xffu),
        f32((v >> 16u) & 0xffu),
        f32((v >> 24u) & 0xffu)
    ) / 255.0;
}
fn q3(c:vec3<f32>) -> vec3<f32> {
    return round(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)) * 255.0) / 255.0;
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let x = gid.x;
    let y = gid.y;
    if (x >= p.w || y >= p.h) { return; }
    let idx = y * p.w + x;
    let s = unpack(src[idx]);
    var col = s.xyz;
    let a8 = round(s.w * 255.0);
    let luma = vec3<f32>(0.2126, 0.7152, 0.0722);

    for (var i = 0u; i < p.n; i = i + 1u) {
        let a4 = chain[2u * i];
        let b4 = chain[2u * i + 1u];
        let invert = a4.x;
        let bright = a4.y;
        let contrast = a4.z;
        let gray = a4.w;
        let sat = b4.x;
        let sepia = b4.y;
        if (invert > 0.0) { col = col + invert * (vec3<f32>(1.0) - 2.0 * col); }
        col = col * bright;
        col = (col - vec3<f32>(0.5)) * contrast + vec3<f32>(0.5);
        if (gray > 0.0) { let l = dot(col, luma); col = col + (vec3<f32>(l) - col) * gray; }
        if (sat != 1.0) { let l = dot(col, luma); col = vec3<f32>(l) + (col - vec3<f32>(l)) * sat; }
        if (sepia > 0.0) {
            let sr = 0.393 * col.x + 0.769 * col.y + 0.189 * col.z;
            let sg = 0.349 * col.x + 0.686 * col.y + 0.168 * col.z;
            let sb = 0.272 * col.x + 0.534 * col.y + 0.131 * col.z;
            col = col + (vec3<f32>(sr, sg, sb) - col) * sepia;
        }
        // hue-rotate omitido aquí: lo gestiona la CPU (conversión HSV de egui).
        col = q3(col);
    }

    var r8 = round(clamp(col.x, 0.0, 1.0) * 255.0);
    var g8 = round(clamp(col.y, 0.0, 1.0) * 255.0);
    var b8 = round(clamp(col.z, 0.0, 1.0) * 255.0);

    if (p.vig > 0.0) {
        let cx = f32(p.w) * 0.5;
        let cy = f32(p.h) * 0.5;
        let radius = max(sqrt(cx * cx + cy * cy), 1.0);
        let dx = f32(x) - cx;
        let dy = f32(y) - cy;
        let t = clamp((sqrt(dx * dx + dy * dy) / radius - 0.42) / 0.58, 0.0, 1.0);
        let k = 1.0 - p.vig * t;
        r8 = trunc(r8 * k);
        g8 = trunc(g8 * k);
        b8 = trunc(b8 * k);
    }

    // Premultiplicado con el alfa original (división entera, como la CPU).
    let af = u32(a8);
    let pr = (u32(r8) * af) / 255u;
    let pg = (u32(g8) * af) / 255u;
    let pb = (u32(b8) * af) / 255u;
    dst[idx] = pr | (pg << 8u) | (pb << 16u) | (af << 24u);
}
"#;

/// `shader` → índice de `mode` del compute de textura. `None` si no es textura.
fn mode_of(shader: &str) -> Option<u32> {
    match shader {
        "vignette" => Some(0),
        "grain" => Some(1),
        "posterize" => Some(2),
        "rgb-split" => Some(3),
        _ => None,
    }
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

impl GpuCtx {
    /// Inicializa el contexto GPU. Devuelve `None` si no hay adaptador (entonces
    /// el compositor usa la reserva en CPU).
    pub fn new() -> Option<Self> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
            apply_limit_buckets: false,
        }))
        .ok()?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("cupcraft-gpu"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        }))
        .ok()?;

        // Pipeline de texturas (3 bindings: uniform, src, dst).
        let tex_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cupcraft-tex"),
            source: wgpu::ShaderSource::Wgsl(TEX_SHADER.into()),
        });
        let tex_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cupcraft-tex-bgl"),
            entries: &[uniform_entry(0), storage_entry(1, true), storage_entry(2, false)],
        });
        let tex_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cupcraft-tex-pl"),
            bind_group_layouts: &[Some(&tex_layout)],
            immediate_size: 0,
        });
        let tex_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cupcraft-tex-pipe"),
            layout: Some(&tex_pl),
            module: &tex_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        // Pipeline de color (4 bindings: uniform, src, dst, chain).
        let col_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("cupcraft-col"),
            source: wgpu::ShaderSource::Wgsl(COL_SHADER.into()),
        });
        let col_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("cupcraft-col-bgl"),
            entries: &[
                uniform_entry(0),
                storage_entry(1, true),
                storage_entry(2, false),
                storage_entry(3, true),
            ],
        });
        let col_pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("cupcraft-col-pl"),
            bind_group_layouts: &[Some(&col_layout)],
            immediate_size: 0,
        });
        let col_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("cupcraft-col-pipe"),
            layout: Some(&col_pl),
            module: &col_module,
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        Some(Self {
            device,
            queue,
            tex_pipeline,
            tex_layout,
            col_pipeline,
            col_layout,
            pool: RefCell::new(Pool::default()),
        })
    }

    /// Mapea los primeros `bytes` del búfer de lectura (residente, puede ser mayor
    /// que el fotograma actual) y copia en `out`. `false` si falla.
    fn finish_readback(&self, readback: &wgpu::Buffer, bytes: u64, out: &mut [u8]) -> bool {
        let slice = readback.slice(..bytes);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res.is_ok());
        });
        let _ = self
            .device
            .poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        let ok = rx.recv().unwrap_or(false);
        let mut done = false;
        if ok {
            if let Ok(data) = slice.get_mapped_range() {
                out.copy_from_slice(&data);
                drop(data);
                done = true;
            }
        }
        readback.unmap();
        done
    }

    /// Ejecuta una pasada de textura sobre `buf` (RGBA8, `w*h*4`). `false` si el
    /// shader no es de textura o el tamaño no cuadra → el llamante usa la CPU.
    pub fn run_texture(&self, shader: &str, buf: &mut [u8], w: u32, h: u32, time: f64, amount: f64) -> bool {
        let Some(mode) = mode_of(shader) else { return false };
        let bytes = (w as u64) * (h as u64) * 4;
        if w == 0 || h == 0 || buf.len() as u64 != bytes {
            return false;
        }
        let params = TexParams {
            w,
            h,
            mode,
            _pad0: 0,
            time: time as f32,
            amount: amount as f32,
            _pad1: 0.0,
            _pad2: 0.0,
        };
        let mut pool = self.pool.borrow_mut();
        let pool = &mut *pool;
        let uniform = pool.uniform.ensure(
            &self.device,
            32,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            "cupcraft-uni",
        );
        self.queue.write_buffer(uniform, 0, bytemuck::bytes_of(&params));
        let src = pool.src.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            "cupcraft-src",
        );
        self.queue.write_buffer(src, 0, buf);
        let dst = pool.dst.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            "cupcraft-dst",
        );
        let readback = pool.readback.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            "cupcraft-read",
        );
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cupcraft-tex-bg"),
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: src.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dst.as_entire_binding() },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("cupcraft-tex-enc") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cupcraft-tex-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.tex_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(dst, 0, readback, 0, bytes);
        self.queue.submit(Some(encoder.finish()));
        self.finish_readback(readback, bytes, buf)
    }

    /// Ejecuta la cadena de color + viñeta sobre `src` (RGBA8, `w*h*4`) y escribe
    /// el resultado premultiplicado en `out` (mismo tamaño). `chain`: eslabones
    /// como `[invert, brightness, contrast, grayscale, saturate, sepia, opacity, _]`.
    /// `false` si el tamaño no cuadra → el llamante usa la CPU.
    pub fn run_color(&self, w: u32, h: u32, src: &[u8], out: &mut [u8], chain: &[[f32; 8]], vig: f32) -> bool {
        let bytes = (w as u64) * (h as u64) * 4;
        if w == 0 || h == 0 || src.len() as u64 != bytes || out.len() as u64 != bytes {
            return false;
        }
        let params = ColParams {
            w,
            h,
            n: chain.len() as u32,
            _pad0: 0,
            vig,
            _pad1: 0.0,
            _pad2: 0.0,
            _pad3: 0.0,
        };
        // La cadena viaja como pares de vec4 (8 floats por eslabón). El búfer de
        // almacenamiento no puede ser de tamaño cero: con cadena vacía (solo
        // viñeta) se envía un eslabón nulo y `n` queda en 0.
        let mut chain_data: Vec<f32> = Vec::with_capacity(chain.len().max(1) * 8);
        for fx in chain {
            chain_data.extend_from_slice(fx);
        }
        if chain_data.is_empty() {
            chain_data.extend_from_slice(&[0.0f32; 8]);
        }

        let chain_bytes = (chain_data.len() * 4) as u64;
        let mut pool = self.pool.borrow_mut();
        let pool = &mut *pool;
        let uniform = pool.uniform.ensure(
            &self.device,
            32,
            wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            "cupcraft-uni",
        );
        self.queue.write_buffer(uniform, 0, bytemuck::bytes_of(&params));
        let src_buf = pool.src.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            "cupcraft-src",
        );
        self.queue.write_buffer(src_buf, 0, src);
        let chain_buf = pool.chain.ensure(
            &self.device,
            chain_bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            "cupcraft-chain",
        );
        self.queue.write_buffer(chain_buf, 0, bytemuck::cast_slice(&chain_data));
        let dst = pool.dst.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            "cupcraft-dst",
        );
        let readback = pool.readback.ensure(
            &self.device,
            bytes,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            "cupcraft-read",
        );
        let bind = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cupcraft-col-bg"),
            layout: &self.col_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: src_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: dst.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 3, resource: chain_buf.as_entire_binding() },
            ],
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("cupcraft-col-enc") });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cupcraft-col-pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.col_pipeline);
            pass.set_bind_group(0, &bind, &[]);
            pass.dispatch_workgroups(w.div_ceil(8), h.div_ceil(8), 1);
        }
        encoder.copy_buffer_to_buffer(dst, 0, readback, 0, bytes);
        self.queue.submit(Some(encoder.finish()));
        self.finish_readback(readback, bytes, out)
    }
}
