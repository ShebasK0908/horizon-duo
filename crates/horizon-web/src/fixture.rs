use crate::render_frame::frame;
use horizon_client_render::{
    assets::RenderAssets,
    gfx::{Renderer, panorama},
    graphics::GraphicsApi,
    hud::HudBatch,
    mesher::{SectionMesh, TerrainVertex},
};
use horizon_core::position::BlockPos;
use std::sync::Arc;
use wasm_bindgen::prelude::*;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::Window,
};

struct RenderFixture {
    renderer: Renderer,
    window: Arc<Window>,
    frames: u64,
}

#[wasm_bindgen]
#[allow(deprecated)]
pub async fn run_render_fixture() -> Result<(), JsValue> {
    run_fixture(None, mesh()).await
}

#[wasm_bindgen]
pub async fn run_owned_resources(bytes: Vec<u8>) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    let imported = crate::resources::ImportedResources::from_zip(bytes).map_err(js_error)?;
    let assets = imported.assets;
    let render = imported.render;
    let mut mesh = SectionMesh::default();
    for (name, at) in [
        ("minecraft:stone", [0, 0, 0]),
        ("minecraft:oak_planks", [1, 0, 0]),
        ("minecraft:oak_stairs", [0, 1, 0]),
    ] {
        let state = assets
            .registry
            .default_state(name)
            .ok_or_else(|| js_error("Missing registry state"))?;
        let pos = BlockPos::new(at[0], at[1], at[2]);
        for (_, model) in render.model_catalog.collect(state, pos).map_err(js_error)? {
            for face in &model.faces {
                let instance = horizon_client_render::models::instance(face, pos);
                let color = (255u32 << 24) | (u32::from((instance.shade * 255.) as u8) * 0x010101);
                for (vertex, uv) in face.quad.positions.into_iter().zip(face.quad.uv) {
                    let vertex = std::array::from_fn(|i| vertex[i] + at[i] as f32);
                    mesh.solid.push(TerrainVertex::new(
                        vertex,
                        uv,
                        face.layer as u16,
                        color,
                        0xf0,
                    ));
                }
            }
        }
    }
    mesh.sort_solid();
    run_fixture(Some(render), mesh).await
}

#[allow(deprecated)]
async fn run_fixture(render: Option<RenderAssets>, mesh: SectionMesh) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    let events = EventLoop::new().map_err(js_error)?;
    let window = Arc::new(
        events
            .create_window(
                Window::default_attributes()
                    .with_title("Horizon renderer fixture")
                    .with_inner_size(winit::dpi::PhysicalSize::new(960, 640)),
            )
            .map_err(js_error)?,
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = window.canvas().ok_or_else(|| js_error("No canvas"))?;
    canvas.set_id("horizon-canvas");
    canvas.set_width(960);
    canvas.set_height(640);
    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(960, 640));
    document.body().unwrap().append_child(&canvas)?;
    let white = [255u8; 4];
    let faces = panorama::Faces {
        size: 1,
        pixels: [70, 100, 140, 255].repeat(6),
    };
    let (pixels, size, minimum, layers, mips, animations) = match &render {
        Some(render) => (
            render.pixels.as_slice(),
            render.sprite_size,
            render.sprite_minimum_size,
            render.texture_layers,
            render.sprite_mips.as_slice(),
            render.animations.as_slice(),
        ),
        None => (white.as_slice(), 1, 1, 1, &[][..], &[][..]),
    };
    let mut renderer = Renderer::new(
        window.clone(),
        false,
        pixels,
        size,
        minimum,
        layers,
        mips,
        animations,
        (&white, 1, 1),
        (&white, 1, 1),
        (&white, 1, 1),
        (&white, 1),
        &faces,
        None,
        None,
        GraphicsApi::Auto,
    )
    .await
    .map_err(js_error)?;
    renderer.upload((0, 8, 0), &mesh);
    let mut fixture = RenderFixture {
        renderer,
        window,
        frames: 0,
    };
    events.spawn(move |event, events| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => events.exit(),
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            fixture.renderer.resize(size.width, size.height);
        }
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            let hud = HudBatch::new([960, 640]);
            let frame = frame(&hud, fixture.frames);
            match fixture.renderer.render(&frame) {
                Ok(true) => {
                    fixture.frames += 1;
                    if fixture.frames == 3 {
                        let body = web_sys::window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .body()
                            .unwrap();
                        body.set_attribute("data-horizon-frames", "3").unwrap();
                        body.set_attribute(
                            "data-horizon-quads",
                            &mesh.solid.len().div_ceil(4).to_string(),
                        )
                        .unwrap();
                    }
                }
                Ok(false) => {}
                Err(error) => panic!("Horizon render failed: {error}"),
            }
        }
        Event::AboutToWait => fixture.window.request_redraw(),
        _ => {}
    });
    Ok(())
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}

fn mesh() -> SectionMesh {
    let mut mesh = SectionMesh::default();
    let faces = [
        [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
        [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
        [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
        [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
        [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
        [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    ];
    for (offset, color) in [
        ([0., 0., 0.], 0xff709940),
        ([1., 0., 0.], 0xffcc8844),
        ([0., 1., 0.], 0xff4488cc),
    ] {
        for face in faces {
            for (pos, uv) in face
                .into_iter()
                .zip([[0., 1.], [1., 1.], [1., 0.], [0., 0.]])
            {
                mesh.solid.push(TerrainVertex::new(
                    std::array::from_fn(|i| pos[i] + offset[i]),
                    uv,
                    0,
                    color,
                    0xf0,
                ));
            }
        }
    }
    mesh.sort_solid();
    mesh
}
