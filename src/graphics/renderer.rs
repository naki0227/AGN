use wgpu::util::DeviceExt;
use lyon::tessellation::*;
use glyphon::{FontSystem, SwashCache, TextAtlas, TextRenderer, Resolution};

pub struct Renderer {
    // Lyon Tessellator
    geometry: VertexBuffers<GpuVertex, u16>,
    tessellator: FillTessellator,
    
    // Text Renderer (Glyphon)
    font_system: FontSystem,
    swash_cache: SwashCache,

    _text_viewport: Option<Resolution>,
    text_atlas: Option<TextAtlas>,
    text_renderer: Option<TextRenderer>,
    
    // Batching
    pub batches: Vec<DrawBatch>,
    
    // Text Queue
    pub text_queue: Vec<TextDrawCommand>,
}

pub struct TextDrawCommand {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub color: [f32; 4],
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct DrawBatch {
    pub index_start: u32,
    pub index_count: u32,
    pub texture_key: Option<String>, // None = White Texture, Some(key) = Image
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GpuVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub effect_flags: u32,
}

impl Renderer {
    pub fn new(_device: &wgpu::Device, _queue: &wgpu::Queue, _format: wgpu::TextureFormat) -> Self {
        Self {
            geometry: VertexBuffers::new(),
            tessellator: FillTessellator::new(),
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            _text_viewport: None,
            text_atlas: None,
            text_renderer: None,
            batches: Vec::new(),
            text_queue: Vec::new(),
        }
    }

    fn update_batch(&mut self, added_indices: u32, texture_key: Option<&str>) {
        // If batches empty, start one
        if self.batches.is_empty() {
             self.batches.push(DrawBatch {
                 index_start: 0,
                 index_count: added_indices,
                 texture_key: texture_key.map(|s| s.to_string()),
             });
             return;
        }

        // Check last batch
        let last_idx = self.batches.len() - 1;
        let last_key = &self.batches[last_idx].texture_key;
        
        let match_key = match (last_key, texture_key) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        };
        
        if match_key {
            self.batches[last_idx].index_count += added_indices;
        } else {
            let start = self.batches[last_idx].index_start + self.batches[last_idx].index_count;
            self.batches.push(DrawBatch {
                index_start: start,
                index_count: added_indices,
                texture_key: texture_key.map(|s| s.to_string()),
            });
        }
    }

    pub fn init_text(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) {
         let mut text_atlas = TextAtlas::new(device, queue, format);
         let text_renderer = TextRenderer::new(&mut text_atlas, &device, wgpu::MultisampleState::default(), None);
         
         self.text_atlas = Some(text_atlas);
         self.text_renderer = Some(text_renderer);
    }

    pub fn begin(&mut self) {
        self.geometry.vertices.clear();
        self.geometry.indices.clear();
        self.batches.clear();
        self.text_queue.clear();
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) {
        let options = FillOptions::default();
        let start_indices = self.geometry.indices.len() as u32;

        let mut builder = BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| {
            GpuVertex {
                position: [vertex.position().x, vertex.position().y],
                color,
                uv: [0.0, 0.0],
                effect_flags: 0,
            }
        });

        let mut path_builder = lyon::path::Path::builder();
        path_builder.add_rectangle(
            &lyon::math::Box2D::new(lyon::math::point(x, y), lyon::math::point(x + w, y + h)),
            lyon::path::Winding::Positive,
        );
        let path = path_builder.build();

        let _ = self.tessellator.tessellate_path(
            &path,
            &options,
            &mut builder,
        ).unwrap();
        
        // Builder borrow ends here? Rust NLL should handle it if not used.
        // Explicitly drop builder to be safe or rely on NLL.
        // But builder is stored in local variable.
        drop(builder); 
        
        let end_indices = self.geometry.indices.len() as u32;
        
        self.update_batch(end_indices - start_indices, None);
    }

    pub fn draw_rounded_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, color: [f32; 4], effect_flags: u32) {
        let options = FillOptions::default();
        let start_indices = self.geometry.indices.len() as u32;

        let mut builder = BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| {
            // Calculate UV based on bounding box for effects
            let px = vertex.position().x;
            let py = vertex.position().y;
            let u = (px - x) / w;
            let v = (py - y) / h;

            GpuVertex {
                position: [px, py],
                color,
                uv: [u, v], // Pass UV for shaders
                effect_flags,
            }
        });

        let mut path_builder = lyon::path::Path::builder();
        path_builder.add_rounded_rectangle(
            &lyon::math::Box2D::new(lyon::math::point(x, y), lyon::math::point(x + w, y + h)),
            &lyon::path::builder::BorderRadii::new(radius),
            lyon::path::Winding::Positive,
        );
        let path = path_builder.build();

        let _ = self.tessellator.tessellate_path(
            &path,
            &options,
            &mut builder,
        ).unwrap();
        
        drop(builder);
        let end_indices = self.geometry.indices.len() as u32;
        
        self.update_batch(end_indices - start_indices, None);
    }

    pub fn draw_texture_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        // Draw a rect with UVs 0..1
        // We manually push vertices/indices or use tessellator?
        // Tessellator gives complex geometry.
        // For a simple image quad, manual push is more efficient but harder to mix with lyon buffers.
        // Let's use tessellator with specific UV logic? 
        // Lyon doesn't support custom attributes easily in builder unless we map position to UV.
        // Hack: Map normalized position to UV.
        
        let color = [1.0, 1.0, 1.0, 1.0]; // White tint
        let options = FillOptions::default();
        let start_indices = self.geometry.indices.len() as u32;
        let mut builder = BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| {
            // How to get UV? Vertex position is absolute pixels.
            // We need to know the bounds (x,y,w,h) to normalize.
            // But the closure doesn't capture them easily if we use generic builder.
            // Wait, we CAN capture x,y,w,h in closure!
            
            let px = vertex.position().x;
            let py = vertex.position().y;
            let u = (px - x) / w;
            let v = (py - y) / h;
            
            GpuVertex {
                position: [px, py],
                color,
                uv: [u, v],
                effect_flags: 0,
            }
        });

        let mut path_builder = lyon::path::Path::builder();
        path_builder.add_rectangle(
            &lyon::math::Box2D::new(lyon::math::point(x, y), lyon::math::point(x + w, y + h)),
            lyon::path::Winding::Positive,
        );
        let path = path_builder.build();

        let _ = self.tessellator.tessellate_path(
            &path,
            &options,
            &mut builder,
        ).unwrap();
        let end_indices = self.geometry.indices.len() as u32;
        
        // Assume explicit texture methods pass key, or overload?
        // Let's modify signature or add `draw_texture_rect_with_key`.
        // For now, this fallback uses "white texture" (None) because signature has no key.
        // We will change signature in next step or use updating batch with logic.
        // But wait, `draw_texture_rect` implies it USES a texture.
        // If we want to support existing code, update signature or create new method.
        // Let's assume we pass key.
        // Since `draw_texture_rect` was unused, we can change signature freely.
        self.update_batch(end_indices - start_indices, None); 
    }

    pub fn draw_image(&mut self, x: f32, y: f32, w: f32, h: f32, texture_key: &str, effect_flags: u32) {
        let color = [1.0, 1.0, 1.0, 1.0];
        let options = FillOptions::default();
        let start_indices = self.geometry.indices.len() as u32;

        let mut builder = BuffersBuilder::new(&mut self.geometry, |vertex: FillVertex| {
            let px = vertex.position().x;
            let py = vertex.position().y;
            // UVs need exact mapping
            let u = (px - x) / w;
            let v = (py - y) / h;
            GpuVertex {
                position: [px, py],
                color,
                uv: [u, v],
                effect_flags,
            }
        });

        let mut path_builder = lyon::path::Path::builder();
        path_builder.add_rectangle(
            &lyon::math::Box2D::new(lyon::math::point(x, y), lyon::math::point(x + w, y + h)),
            lyon::path::Winding::Positive,
        );
        let path = path_builder.build();

        let _ = self.tessellator.tessellate_path(
            &path,
            &options,
            &mut builder,
        ).unwrap();
        
        drop(builder);
        let end_indices = self.geometry.indices.len() as u32;
        
        self.update_batch(end_indices - start_indices, Some(texture_key));
    }

    pub fn draw_shadow_rect(&mut self, x: f32, y: f32, w: f32, h: f32, radius: f32, depth: f32) {
        if depth <= 0.0 { return; }
        // Simple multi-pass shadow
        let shadow_color = [0.0, 0.0, 0.0, 0.1];
        let offset = depth * 0.5;
        self.draw_rounded_rect(x + offset, y + offset, w, h, radius, shadow_color, 0);
        if depth > 5.0 {
             self.draw_rounded_rect(x + offset + 2.0, y + offset + 2.0, w, h, radius, [0.0, 0.0, 0.0, 0.05], 0);
        }
    }
    
    // Returns vertex buffer and index buffer
    pub fn get_buffers(&self, device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&self.geometry.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&self.geometry.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buf, index_buf, self.geometry.indices.len() as u32)
    }

    pub fn draw_text(&mut self, text: &str, x: f32, y: f32, color: [f32; 4], size: f32) {
        self.text_queue.push(TextDrawCommand {
            text: text.to_string(),
            x,
            y,
            color,
            size,
        });
    }

    pub fn render_text<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>, device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) {
        if self.text_renderer.is_none() { return; }
        if self.text_queue.is_empty() { return; }

        let renderer = self.text_renderer.as_mut().unwrap();
        let font_system = &mut self.font_system;
        let atlas = self.text_atlas.as_mut().unwrap();

        let mut buffers = Vec::new();
        
        // Populate buffers
        for cmd in &self.text_queue {
            let mut buffer = glyphon::Buffer::new(font_system, glyphon::Metrics::new(cmd.size, cmd.size * 1.2));
            buffer.set_size(font_system, width as f32, height as f32);
            buffer.set_text(
                font_system, 
                &cmd.text, 
                glyphon::Attrs::new().color(glyphon::Color::rgba((cmd.color[0]*255.0) as u8, (cmd.color[1]*255.0) as u8, (cmd.color[2]*255.0) as u8, (cmd.color[3]*255.0) as u8)).family(glyphon::Family::SansSerif), 
                glyphon::Shaping::Advanced
            );
            buffers.push((buffer, cmd.x, cmd.y));
        }

        // Create TextAreas
        // Note: Buffer needs to be moved or referenced? TextArea takes &Buffer.
        // We created buffers in a Vec, so we can reference them.
        let text_areas: Vec<glyphon::TextArea> = buffers.iter().map(|(buf, x, y)| {
            glyphon::TextArea {
                buffer: buf,
                left: *x,
                top: *y,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                },
                default_color: glyphon::Color::rgb(255, 255, 255),
            }
        }).collect();

        renderer.prepare(
            device,
            queue,
            font_system,
            atlas,
            Resolution { width, height },
            text_areas.iter().cloned(), 
            &mut self.swash_cache,
        ).unwrap();

        renderer.render(atlas, pass).unwrap();
    }
}
