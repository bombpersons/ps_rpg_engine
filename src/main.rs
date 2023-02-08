use std::{path::{Path, PathBuf}, collections::HashMap};
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder}, event::{Event, WindowEvent}};
use bevy_ecs::prelude::*;

mod renderer;

#[derive(StageLabel)]
pub struct RenderStage;

#[tokio::main]
async fn main() {
    // Initialize the logger.
    env_logger::init();

    // Create a window.
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    
    // Create the bevy world.
    let mut world = World::new();

    // Add a test field.
    world.insert_resource(
     renderer::field::Field::from_gltf("test_field", "test_field_depth", Path::new("fields/test_field.gltf"))
    );

    // Add texture manager.
    let textures = {
      let mut map = HashMap::new();
      map.insert("test_field".to_string(), PathBuf::from(r"fields/test_field.png"));

      map
    };
    world.insert_resource(renderer::texture_manager::TextureManager::new(textures));

    // Create a schedule for the renderer.
    let mut render_schedule = Schedule::default();

    // Add render system.
    let render_systemset = renderer::init(&mut world, &window);
    render_schedule.add_stage(RenderStage, SystemStage::single_threaded()
        .with_system_set(render_systemset)
    );

    // Run the event loop.
    event_loop.run(move |event, _, control_flow| {
        log::debug!("{:?}", event);

        match event {
            // Draw
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                render_schedule.run(&mut world);
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
                  world.resource_scope(|_, mut viewport: Mut<renderer::Viewport>| {
                    viewport.set_size((*physical_size).width, (*physical_size).height);
                  });
                },

                WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                  world.resource_scope(|_, mut viewport: Mut<renderer::Viewport>| {
                    viewport.set_size((**new_inner_size).width, (**new_inner_size).height);
                  });
                },

                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                _  => {}
            },
            _ => {}
        }
    });

    //run_game_window().await;
}
