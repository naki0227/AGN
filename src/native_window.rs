use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use crate::symbol_table::{SymbolTable, Value};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use crate::interpreter::RuntimeMessage;
use crate::graphics::state::State;

pub fn run_native_window(rx: Receiver<RuntimeMessage>, symbol_table: Arc<Mutex<SymbolTable>>) {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("AGN Native Window (wgpu)")
        .build(&event_loop)
        .unwrap();

    // Initialize wgpu State
    let mut state = pollster::block_on(State::new(&window));

    println!("[Native] Window started with wgpu backend.");

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);

        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        let table = symbol_table.lock().unwrap();
                        // Find a root component? Or check specific variable "Root" or "Card" or just iterate components?
                        // For demo, we might look for "カード" (Card) or "Window" or generic iteration.
                        // Ideally we pass the whole table or specific root.
                        // Let's assume there is a variable named "Screen" or we just render the first component found?
                        // "カード" is used in the demo.
                        // Let's iterate and find the first Value::Component for now.
                        let mut root: Option<Value> = None;
                        for (k, v) in &table.symbols {
                            if let Value::Component { .. } = v {
                                // If it has children and is top level?
                                // Let's simplify: Render variable "カード" if exists.
                                if k == "カード" || k == "Card" {
                                    root = Some(v.clone());
                                    break;
                                }
                            }
                        }
                        // If no card, maybe render "Window"? 
                        // If root is still None, take ANY component.
                        if root.is_none() {
                             for (_, v) in &table.symbols {
                                 if let Value::Component { .. } = v {
                                     root = Some(v.clone());
                                     break;
                                 }
                             }
                        }
                        
                        match state.render(root.as_ref()) {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    }
                    WindowEvent::CursorMoved { position, .. } => {
                        state.update_cursor(position.x as f32, position.y as f32);
                        window.request_redraw();
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                // Check channel for new messages
                while let Ok(msg) = rx.try_recv() {
                    match msg {
                        RuntimeMessage::String(s) => {
                            println!("[Native] Screen Updated: {}", s);
                            window.set_title(&format!("AGN Screen: {}", s));
                        },
                        RuntimeMessage::Animate(anim) => {
                            state.animation_controller.add_animation(anim);
                        },
                        RuntimeMessage::RegisterEvent(target, event, anims) => {
                            println!("[Native] Registered {} event for {}", event, target);
                            state.register_event(target, event, anims);
                        },
                        RuntimeMessage::LoadImage(target, path) => {
                            state.load_image(target, path);
                        }
                    }
                }
                
                // Update animations
                state.update();
                
                window.request_redraw();
            },
            _ => ()
        }
    }).unwrap();
}
