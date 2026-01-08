use std::sync::Arc;
use std::collections::HashMap;
use winit::window::Window;

use crate::graphics::renderer::{Renderer, GpuVertex};
use crate::graphics::layout::LayoutEngine;
use crate::graphics::animation::{AnimationController, Animation};
use crate::symbol_table::{SymbolTable, Value};
use wgpu::util::DeviceExt;
use image::GenericImageView;

pub struct State {
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub render_pipeline: wgpu::RenderPipeline,
    
    pub renderer: Renderer,
    pub layout_engine: LayoutEngine,
    pub symbol_table: Arc<std::sync::Mutex<SymbolTable>>,
    pub animation_controller: AnimationController,
    
    // Interaction State
    pub event_handlers: HashMap<String, HashMap<String, Vec<Animation>>>,
    pub hovered_component: Option<String>,
    pub cursor_pos: Option<(f32, f32)>,
    
    // Cache
    pub layout_rects: Vec<(f32, f32, f32, f32, String)>, // x,y,w,h, label (only for hit testing)
    
    // Resources
    pub bind_group: wgpu::BindGroup,
    pub screen_size_buffer: wgpu::Buffer,
    pub white_texture: wgpu::Texture,
    pub white_texture_view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group_layout: wgpu::BindGroupLayout,
    
    // Image Cache
    pub image_bind_groups: HashMap<String, wgpu::BindGroup>, // Key: Target (Component Name) or Path?
    // Let's key by target name so component usage is easy.
}

impl State {
    // Creating some of the wgpu types requires async code
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        
        // Use Arc::new to satisfy 'static requirement for Surface if needed, 
        // but winit 0.29 + wgpu 0.19 allows simplified surface creation usually.
        // However, referencing window safely often requires keeping window alive or using Arc.
        // Here we assume window lives longer than State or unsafe pointer usage in creating surface (common in wgpu examples).
        // Actually wgpu 0.19 surface target takes msg.
        // For simplicity, we use create_surface_unsafe or similar if lifetime issues arise, 
        // but typically instance.create_surface(window) works if state owned by window loop.
        // But create_surface returns Surface<'window>, which might be tricky struct field.
        // We use Surface<'static> by using unsafe or Arc<Window>. 
        // For this prototype, we'll try standard safe creation and see if compiler complains about lifetimes.
        // To avoid complex lifetimes in State struct, we might need unsafe transmute or Arc.
        // Let's rely on `wgpu::SurfaceTarget` being compatible.
        
        let surface = unsafe { 
            let target = wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap();
            instance.create_surface_unsafe(target).unwrap() 
        };

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code assumes sRGB output?
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
            
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        
        surface.configure(&device, &config);

        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // Create resources
        
        // 1. Uniform Buffer (Screen Size)
        let screen_size_data = [size.width as f32, size.height as f32];
        let screen_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Screen Size Buffer"),
            contents: bytemuck::cast_slice(&screen_size_data),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // 2. Texture (1x1 White)
        let texture_size = wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 };
        let white_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("White Texture"),
            view_formats: &[],
        });
        queue.write_texture(
             wgpu::ImageCopyTexture { texture: &white_texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
             &[255, 255, 255, 255],
             wgpu::ImageDataLayout { offset: 0, bytes_per_row: Some(4), rows_per_image: Some(1) },
             texture_size,
        );
        let white_texture_view = white_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // 3. Bind Group
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture { multisampled: false, view_dimension: wgpu::TextureViewDimension::D2, sample_type: wgpu::TextureSampleType::Float { filterable: true } },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
            label: Some("texture_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&white_texture_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: screen_size_buffer.as_entire_binding() },
            ],
            label: Some("diffuse_bind_group"),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Vertex Buffer Layout
        let vertex_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 0, shader_location: 0 }, // position
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 8, shader_location: 1 }, // color
                wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 }, // uv
            ],
        };

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", 
                buffers: &[vertex_buffer_layout],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let mut renderer = Renderer::new(&device, &queue, config.format);
        // Initialize text renderer
        renderer.init_text(&device, &queue, config.format);
        
        let layout_engine = LayoutEngine::new();
        let symbol_table = Arc::new(std::sync::Mutex::new(SymbolTable::new()));
        let animation_controller = AnimationController::new();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            renderer,
            layout_engine,
            symbol_table,
            animation_controller,
            event_handlers: HashMap::new(),
            hovered_component: None,
            cursor_pos: None,
            layout_rects: Vec::new(),
            
            // Resources
            bind_group,
            screen_size_buffer,
            white_texture,
            white_texture_view,
            sampler,
            bind_group_layout,
            image_bind_groups: HashMap::new(),
        }
    }

    pub fn register_event(&mut self, target: String, event: String, animations: Vec<Animation>) {
        let handlers = self.event_handlers.entry(target).or_insert_with(HashMap::new);
        handlers.insert(event, animations);
    }
    
    pub fn load_image(&mut self, target: String, path: String) {
        println!("[State] Loading image for {}: {}", target, path);
        // Load image using `image` crate
        // Note: For native, use File I/O. For WASM, this would be different (fetch).
        // Since we are Native target currently:
        let img = match image::open(&path) {
            Ok(i) => i,
            Err(e) => {
                eprintln!("[State] Failed to load image '{}': {}", path, e);
                return;
            }
        };
        
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        
        // Create Texture
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some(&path),
            view_formats: &[],
        });
        
        self.queue.write_texture(
             wgpu::ImageCopyTexture {
                 texture: &texture,
                 mip_level: 0,
                 origin: wgpu::Origin3d::ZERO,
                 aspect: wgpu::TextureAspect::All,
             },
             &rgba,
             wgpu::ImageDataLayout {
                 offset: 0,
                 bytes_per_row: Some(4 * dimensions.0),
                 rows_per_image: Some(dimensions.1),
             },
             texture_size,
        );
        
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create Bind Group (using dedicated layout for diffuse or shared?)
        // We reuse the existing layout from `new()`: `bind_group_layout`.
        
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.screen_size_buffer.as_entire_binding(),
                },
            ],
            label: Some(&format!("bind_group_{}", path)),
        });
        
        self.image_bind_groups.insert(path, bind_group);
    }
    
    pub fn update_cursor(&mut self, x: f32, y: f32) {
        self.cursor_pos = Some((x, y));
        self.check_hover();
    }
    
    pub fn check_hover(&mut self) {
        if let Some((mx, my)) = self.cursor_pos {
             let mut found_hover: Option<String> = None;
             for (x, y, w, h, label) in &self.layout_rects {
                 if mx >= *x && mx <= x + w && my >= *y && my <= y + h {
                     found_hover = Some(label.clone());
                     break; // Front-to-back assumption? Layout returns parent first?
                 }
             }
             
             if found_hover != self.hovered_component {
                 // Mouse leave old
                 // (Optional: reverse animation? Not implemented)
                 
                 // Mouse enter new
                  if let Some(target) = &found_hover {
                      if let Some(handlers) = self.event_handlers.get(target) {
                          if let Some(anims) = handlers.get("hover") {
                              println!("[State] Hover Triggered on {}", target);
                              for anim in anims {
                                 self.animation_controller.add_animation(anim.clone());
                              }
                          }
                      }
                  }
                 
                 self.hovered_component = found_hover;
             }
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            // Update Uniform
            let screen_size_data = [new_size.width as f32, new_size.height as f32];
            self.queue.write_buffer(&self.screen_size_buffer, 0, bytemuck::cast_slice(&screen_size_data));
        }
    }

    pub fn update(&mut self) {
        let _updates = self.animation_controller.update();
        // In a real implementation, we would apply _updates to the Component Tree here.
        // For Phase 11 prototype, we might just repaint, assuming LayoutEngine reads animated values
        // if we hook them up. 
        // Or, we render using values from AnimationController if they match target_id.
    }

    pub fn render(&mut self, ui_root: Option<&Value>) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 1. Layout
        let mut layout_rects = Vec::new();
        if let Some(root_val) = ui_root {
             layout_rects = self.layout_engine.compute_layout(
                 root_val, 
                 self.size.width as f32, 
                 self.size.height as f32
             );
        }

        // 2. Tessellate
        self.renderer.begin();
        
        // Draw background
        // Check for "背景" or "Background" in symbol table
        {
            let table = self.symbol_table.lock().unwrap();
            let bg_path = if let Some(Value::Image(path)) = table.lookup("背景") {
                Some(path.clone())
            } else if let Some(Value::Image(path)) = table.lookup("Background") {
                Some(path.clone())
            } else {
                None
            };
            
            if let Some(path) = bg_path {
                // Determine if we need to load it (if not in image_bind_groups)
                // However, render can't be async or easily mutate in a way that blocks?
                // `load_image` is sync (using image crate).
                // But we hold table lock. `load_image` doesn't need table lock.
                // We drop table lock first.
                drop(table);
                
                if !self.image_bind_groups.contains_key(&path) {
                    self.load_image("Background".to_string(), path.clone());
                }
                
                // Draw Full Screen
                self.renderer.draw_image(0.0, 0.0, self.size.width as f32, self.size.height as f32, &path);
            }
        }

        // Store rects for hit testing (need to clone label)
        self.layout_rects = layout_rects.iter().filter_map(|(x,y,w,h,v)| {
             if let Value::Component { label: Some(l), .. } = v {
                 Some((*x, *y, *w, *h, l.clone()))
             } else {
                 None
             }
        }).collect();

        for (x, y, w, h, val) in layout_rects {
            match val {
                Value::Image(path) => {
                     // Draw Image
                     self.renderer.draw_image(x, y, w, h, &path);
                }
                Value::Component { style, ty: _, label, .. } => {
                    // Check for animated properties
                    let mut shadow_depth = 0.0;
                    let mut color_override = None;
                    
                    if let Some(l) = label {
                         if let Some(d) = self.animation_controller.get_value(l.as_str(), "shadow") {
                             shadow_depth = d;
                         }
                         if let Some(d) = self.animation_controller.get_value(l.as_str(), "背景") {
                             // Interpolate White -> Cyan based on d (0.0 -> 1.0)
                             // White: [1.0, 1.0, 1.0]
                             // Cyan: [0.0, 1.0, 1.0] (roughly)
                             let r = 1.0 - d.min(1.0).max(0.0);
                             let g = 1.0;
                             let b = 1.0;
                             color_override = Some([r, g, b, 1.0]);
                         }
                    }

                    // Resolve base color
                    let base_color = match style.as_str() {
                        "Blue" | "青い" => [0.2, 0.4, 0.8, 1.0],
                        "Red" | "赤い" => [0.8, 0.2, 0.2, 1.0],
                        "Green" | "緑の" => [0.2, 0.8, 0.2, 1.0],
                        "White" | "白い" => [1.0, 1.0, 1.0, 1.0],
                        _ => [0.5, 0.5, 0.5, 1.0],
                    };
                    
                    let color = color_override.unwrap_or(base_color);
                    
                    // Draw Shadow
                    self.renderer.draw_shadow_rect(x, y, w, h, 10.0, shadow_depth);
                    
                    // Draw Component
                    self.renderer.draw_rounded_rect(x, y, w, h, 10.0, color);
                    
                    // Label text? (Not implemented in renderer yet, passing rect is needed)
                }
                Value::Image(_) => {
                    // Placeholder for image (gray box)
                    self.renderer.draw_rect(x, y, w, h, [0.8, 0.8, 0.8, 1.0]);
                }
                Value::String(s) => {
                     // Text logic (Draw placeholder rect for text area debugging)
                     // self.renderer.draw_rect(x, y, w, h, [0.0, 0.0, 0.0, 0.1]);
                     // Draw text using Glyphon (future)
                }
                _ => {}
            }
        }
        
        let (vertex_buf, index_buf, index_count) = self.renderer.get_buffers(&self.device);

        // 3. Render Pass
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });


        {
            let mut _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            
            _render_pass.set_pipeline(&self.render_pipeline);
            _render_pass.set_vertex_buffer(0, vertex_buf.slice(..));
            _render_pass.set_index_buffer(index_buf.slice(..), wgpu::IndexFormat::Uint16);
            
            for batch in &self.renderer.batches {
                // Set texture bind group
                if let Some(key) = &batch.texture_key {
                    if let Some(bg) = self.image_bind_groups.get(key) {
                        _render_pass.set_bind_group(0, bg, &[]);
                    } else {
                        // Fallback to white texture if image missing
                        println!("[State] Missing texture: {}", key);
                        _render_pass.set_bind_group(0, &self.bind_group, &[]);
                    }
                } else {
                    _render_pass.set_bind_group(0, &self.bind_group, &[]);
                }
                
                let range = batch.index_start..(batch.index_start + batch.index_count);
                if !range.is_empty() {
                    _render_pass.draw_indexed(range, 0, 0..1); 
                }
            }
        }

    
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}
