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
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{Window, WindowId},
};

pub(crate) struct RenderFixture {
    renderer: Renderer,
    window: Arc<Window>,
    frames: u64,
    quads: usize,
}

#[wasm_bindgen]
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

async fn run_fixture(render: Option<RenderAssets>, mesh: SectionMesh) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    crate::application::start(crate::application::Startup::Fixture { render, mesh })
}
impl RenderFixture {
    pub(crate) async fn new(
        window: Arc<Window>,
        render: Option<RenderAssets>,
        mesh: SectionMesh,
    ) -> Result<Self, JsValue> {
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
        Ok(Self {
            renderer,
            window,
            frames: 0,
            quads: mesh.solid.len().div_ceil(4),
        })
    }
}
impl ApplicationHandler<crate::application::Ready> for RenderFixture {
    fn resumed(&mut self, _: &ActiveEventLoop) {}
    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => events.exit(),
            WindowEvent::Resized(size) => self.renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let size = self.window.inner_size();
                let hud = HudBatch::new([size.width, size.height]);
                let frame = frame(&hud, self.frames);
                match self.renderer.render(&frame) {
                    Ok(true) => {
                        self.frames += 1;
                        if self.frames == 3 {
                            let body = web_sys::window()
                                .unwrap()
                                .document()
                                .unwrap()
                                .body()
                                .unwrap();
                            body.set_attribute("data-horizon-frames", "3").unwrap();
                            body.set_attribute("data-horizon-quads", &self.quads.to_string())
                                .unwrap();
                        }
                    }
                    Ok(false) => {}
                    Err(error) => {
                        web_sys::window()
                            .unwrap()
                            .document()
                            .unwrap()
                            .get_element_by_id("error")
                            .unwrap()
                            .set_text_content(Some(&format!(
                                "Horizon could not draw the world. Reload to try again. {error}"
                            )));
                        events.exit();
                    }
                }
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, _: &ActiveEventLoop) {
        self.window.request_redraw();
    }
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
