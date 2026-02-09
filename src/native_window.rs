use winit::event::{Event, WindowEvent, ElementState, MouseButton};
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

    let window = Arc::new(window);
    // Initialize wgpu State
    let mut state = pollster::block_on(State::new(window.clone()));

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
                    WindowEvent::MouseInput { state: button_state, button, .. } => {
                        if *button_state == ElementState::Pressed && *button == MouseButton::Left {
                            if let Some((x, y)) = state.cursor_pos {
                                // Default click effect
                                state.spawn_particles(x, y, 5, [1.0, 1.0, 1.0, 0.8]);

                                if let Some(clicked_id) = state.handle_click(x, y) {
                                    println!("[Native] Clicked: {}", clicked_id);
                                    
                                    // Effect: Gold burst for valid click
                                    state.spawn_particles(x, y, 15, [1.0, 0.84, 0.0, 1.0]);

                                    // Spawn interpreter thread to handle event
                                    let symbol_table = symbol_table.clone();
                                    let clicked_id = clicked_id.clone();
                                    std::thread::spawn(move || {
                                        use crate::interpreter::Interpreter;
                                        // We need tokio runtime to run async interpreter methods
                                        match tokio::runtime::Runtime::new() {
                                            Ok(rt) => {
                                                rt.block_on(async {
                                                    use crate::bridge::std_bridge::{StdP2PBridge, StdUIManager};
                                                    let p2p = std::sync::Arc::new(StdP2PBridge);
                                                    let ui = std::sync::Arc::new(StdUIManager);
                                                    let interpreter = Interpreter::with_symbol_table(symbol_table, p2p, ui);
                                                    interpreter.handle_ui_event(&clicked_id).await;
                                                });
                                            }
                                            Err(e) => eprintln!("Failed to create runtime for event: {}", e),
                                        }
                                    });
                                }
                            }
                        }
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
