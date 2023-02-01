use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder}, event::{Event, WindowEvent}};

mod renderer;

// Run the game window. This won't return until the window closes.
pub async fn run_game_window() {
    // Create the window.
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    // Create the renderer.
    let mut renderer = renderer::Renderer::new(&window).await;

    // Run the event loop.
    event_loop.run(move |event, _, control_flow| {
        match event {
            // Draw
            Event::RedrawRequested(window_id) if window_id == window.id() => match renderer.render() {
                Ok(_) => {}
                Err(e) => eprintln!("{:?}", e),
            },

            Event::MainEventsCleared => {
                // Request another draw.
                window.request_redraw();
            },

            Event::WindowEvent {
                ref event,
                window_id
            } if window_id == window.id() => match event {
                // Resized window.
                WindowEvent::Resized(physical_size) => {
                    renderer.resize(*physical_size);
                },

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                    renderer.resize(**new_inner_size);
                },

                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _  => {}
            },
            _ => {}
        }
    });
}

#[tokio::main]
async fn main() {
    run_game_window().await;
}
