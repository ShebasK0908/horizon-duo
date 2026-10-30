use crate::{
    render_frame::frame,
    resources::{ImportedResources, WorldResources},
    socket::ClientConnection,
};
use glam::DVec3;
use horizon_client_render::{
    Result,
    chunk::LevelShape,
    environment::Environment,
    gfx::{Renderer, SectionKey, panorama},
    graphics::GraphicsApi,
    hud::HudBatch,
    mesher::Mesher,
    remote_chunks::ChunkFormat,
    world::World,
};
use horizon_core::biome::BiomeZoom;
use horizon_protocol::{
    client_session::{ClientSession, Event as SessionEvent},
    packets::{Empty, configuration as cfg, play},
    registry::{Dynamic, ceil_log2},
};
use horizon_world::registry::BlockStateId;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use wasm_bindgen::prelude::*;
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::Window,
};

struct BrowserClient {
    renderer: Renderer,
    window: Arc<Window>,
    resources: WorldResources,
    world: World,
    session: ClientSession,
    connection: ClientConnection,
    registries: Dynamic,
    format: ChunkFormat,
    zoom: BiomeZoom,
    environment: Environment,
    camera: DVec3,
    yaw: f32,
    pitch: f32,
    loaded: bool,
    teleported: bool,
    dirty: VecDeque<SectionKey>,
    pending: HashSet<SectionKey>,
    frames: u64,
    sections: u64,
    peers: HashMap<i32, [f64; 3]>,
    last_tick: horizon_client_render::time::Instant,
}
#[wasm_bindgen]
#[allow(deprecated)]
pub async fn connect_world(
    bytes: Vec<u8>,
    bridge: String,
    world_id: String,
    name: String,
) -> std::result::Result<(), JsValue> {
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);
    let resources = WorldResources::new(ImportedResources::from_zip(bytes).map_err(js_error)?)
        .map_err(js_error)?;
    let events = EventLoop::new().map_err(js_error)?;
    let window = Arc::new(
        events
            .create_window(
                Window::default_attributes()
                    .with_title("Horizon")
                    .with_inner_size(winit::dpi::PhysicalSize::new(960, 640)),
            )
            .map_err(js_error)?,
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let canvas = window
        .canvas()
        .ok_or_else(|| js_error("No browser canvas"))?;
    canvas.set_id("horizon-canvas");
    canvas.set_width(960);
    canvas.set_height(640);
    document.body().unwrap().append_child(&canvas)?;
    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(960, 640));
    let render = &resources.imported.render;
    let white = [255u8; 4];
    let faces = panorama::Faces {
        size: 1,
        pixels: [70, 100, 140, 255].repeat(6),
    };
    let renderer = Renderer::new(
        window.clone(),
        false,
        &render.pixels,
        render.sprite_size,
        render.sprite_minimum_size,
        render.texture_layers,
        &render.sprite_mips,
        &render.animations,
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
    let environment =
        Environment::load_dimension_from(&resources.imported.source, "overworld", true)
            .map_err(js_error)?;
    let air = resources
        .imported
        .assets
        .registry
        .default_state("minecraft:air")
        .ok_or_else(|| js_error("No air state"))?;
    let connection = ClientConnection::connect(&bridge)?;
    let (session, start) =
        ClientSession::begin(world_id, 25565, name, resources.identity).map_err(js_error)?;
    let mut client = BrowserClient {
        renderer,
        window,
        resources,
        world: World::new(air, 0),
        session,
        connection,
        registries: Dynamic::default(),
        format: ChunkFormat::default(),
        zoom: BiomeZoom::new(0),
        environment,
        camera: DVec3::ZERO,
        yaw: 0.,
        pitch: 0.,
        loaded: false,
        teleported: false,
        dirty: VecDeque::new(),
        pending: HashSet::new(),
        frames: 0,
        sections: 0,
        peers: HashMap::new(),
        last_tick: horizon_client_render::time::Instant::now(),
    };
    for event in start {
        client.handle(event).map_err(js_error)?;
    }
    events.spawn(move |event, events| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => events.exit(),
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => client.renderer.resize(size.width, size.height),
        Event::WindowEvent {
            event: WindowEvent::RedrawRequested,
            ..
        } => {
            if let Err(error) = client.draw() {
                show_error(&error.to_string());
                events.exit();
            }
        }
        Event::AboutToWait => client.window.request_redraw(),
        _ => {}
    });
    Ok(())
}
fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
fn show_error(error: &str) {
    log::error!("{error}");
    if let Some(element) = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id("error")
    {
        element.set_text_content(Some(error));
    }
}
impl BrowserClient {
    fn handle(&mut self, event: SessionEvent) -> Result<()> {
        match event {
            SessionEvent::Send(bytes) => self.connection.send(bytes)?,
            SessionEvent::Joined { registries, .. } => {
                self.registries = registries;
            }
            SessionEvent::Packet(packet) => self.packet(*packet)?,
        }
        Ok(())
    }
    fn send(&mut self, packet: play::Serverbound) -> Result<()> {
        let event = self.session.send_play(packet)?;
        self.handle(event)
    }
    fn packet(&mut self, packet: play::Clientbound) -> Result<()> {
        use play::Clientbound as C;
        match packet {
            C::AddEntity(packet) => {
                if self
                    .resources
                    .builtin
                    .get("minecraft:entity_type")
                    .and_then(|ids| ids.name(packet.kind))
                    == Some("minecraft:player")
                {
                    self.peers.insert(packet.id, packet.position);
                }
            }
            C::RemoveEntities(packet) => {
                for id in packet.ids {
                    self.peers.remove(&id);
                }
            }
            C::Login(packet) => self.dimension(&packet.spawn)?,
            C::Respawn(packet) => self.dimension(&packet.spawn)?,
            C::PlayerPosition(packet) => {
                let pick = |i, bit| {
                    if packet.relatives & bit != 0 {
                        self.camera[i] + packet.position[i]
                    } else {
                        packet.position[i]
                    }
                };
                self.camera = DVec3::new(
                    pick(0, play::relative::X),
                    pick(1, play::relative::Y),
                    pick(2, play::relative::Z),
                );
                self.yaw = if packet.relatives & play::relative::Y_ROT != 0 {
                    self.yaw + packet.yaw
                } else {
                    packet.yaw
                };
                self.pitch = (if packet.relatives & play::relative::X_ROT != 0 {
                    self.pitch + packet.pitch
                } else {
                    packet.pitch
                })
                .clamp(-90., 90.);
                self.teleported = true;
                self.send(play::Serverbound::AcceptTeleportation(
                    play::AcceptTeleportation {
                        id: packet.teleport,
                        x: self.camera.x,
                        y: self.camera.y,
                        z: self.camera.z,
                        yaw: self.yaw,
                        pitch: self.pitch,
                    },
                ))?;
            }
            C::LevelChunkWithLight(packet) => {
                let chunk = self.format.column(&packet)?;
                let (x, z) = (packet.x, packet.z);
                self.world.chunks.insert((x, z), Arc::new(chunk));
                self.dirty_near(x, z);
            }
            C::LightUpdate(packet) => {
                let (layers, _) = self.format.light(&packet.light);
                if let Some(chunk) = self.world.chunks.get_mut(&(packet.x, packet.z)) {
                    horizon_client_render::light::update_light_layers(
                        Arc::make_mut(chunk),
                        &layers,
                        self.format.has_skylight,
                    );
                    self.dirty_near(packet.x, packet.z);
                }
            }
            C::BlockUpdate(packet) => {
                self.world.set(
                    packet.pos[0],
                    packet.pos[1],
                    packet.pos[2],
                    BlockStateId(packet.state as u32),
                )?;
                self.dirty_near(packet.pos[0] >> 4, packet.pos[2] >> 4);
            }
            C::ChunkBatchFinished(_) => self.send(play::Serverbound::ChunkBatchReceived(
                play::ChunkBatchReceived {
                    chunks_per_tick: 4.,
                },
            ))?,
            C::ForgetLevelChunk(packet) => {
                let [x, z] = packet.pos;
                self.world.chunks.remove(&(x, z));
                self.renderer.remove_columns([(x, z)]);
                self.dirty_near(x, z);
            }
            C::SetTime(packet) => {
                self.world.game_time = packet.game_time;
                if let Some(clock) = packet.clocks.first() {
                    self.world.day_time = clock.total_ticks;
                }
            }
            C::StartConfiguration(_) => {
                return Err(
                    "Server reconfiguration is not implemented in the browser client".into(),
                );
            }
            _ => {}
        }
        Ok(())
    }
    fn dimension(&mut self, spawn: &play::SpawnInfo) -> Result<()> {
        let dimension = self.registries.dimension(spawn.dimension_type);
        let shape = LevelShape::new(
            dimension.min_y,
            dimension.height,
            dimension.logical_height,
            dimension.has_skylight,
        )?;
        self.world = World::new(self.world.air, 0);
        self.world.shape = shape;
        let biome_map = (0..self.registries.len("minecraft:worldgen/biome"))
            .map(|i| {
                self.resources.biomes.id(self
                    .registries
                    .name("minecraft:worldgen/biome", i as i32)
                    .unwrap_or("minecraft:plains"))
            })
            .collect();
        self.format = ChunkFormat::new(
            &dimension,
            biome_map,
            ceil_log2(self.registries.len("minecraft:worldgen/biome")).max(1),
        )
        .in_column(shape.column());
        self.zoom = BiomeZoom::new(spawn.seed);
        self.renderer.clear_sections(shape.column());
        self.dirty.clear();
        self.pending.clear();
        self.loaded = false;
        let name = spawn
            .dimension
            .strip_prefix("minecraft:")
            .ok_or("Only vanilla dimensions are currently supported")?;
        self.environment = Environment::load_dimension_from(
            &self.resources.imported.source,
            name,
            name == "overworld",
        )?;
        self.send(play::Serverbound::ClientInformation(
            cfg::ClientInformation::new(4),
        ))?;
        Ok(())
    }
    fn dirty_near(&mut self, x: i32, z: i32) {
        for dz in -1..=1 {
            for dx in -1..=1 {
                if !self.world.chunks.contains_key(&(x + dx, z + dz)) {
                    continue;
                }
                for sy in 0..self.world.layout().sections() as i32 {
                    let key = (x + dx, sy, z + dz);
                    if self.pending.insert(key) {
                        self.dirty.push_back(key);
                    }
                }
            }
        }
    }
    fn draw(&mut self) -> Result<()> {
        for _ in 0..16 {
            let Some(bytes) = self.connection.receive()? else {
                break;
            };
            for event in self.session.receive(bytes)? {
                self.handle(event)?;
            }
        }
        self.connection.flush()?;
        for _ in 0..2 {
            let Some((x, sy, z)) = self.dirty.pop_front() else {
                break;
            };
            self.pending.remove(&(x, sy, z));
            if !self.world.chunks.contains_key(&(x, z)) {
                continue;
            }
            let region = std::array::from_fn(|dz| {
                std::array::from_fn(|dx| {
                    self.world
                        .chunks
                        .get(&(x + dx as i32 - 1, z + dz as i32 - 1))
                        .map(AsRef::as_ref)
                })
            });
            let mut mesher = Mesher {
                nether_shading: !self.world.shape.sky_light,
                assets: &self.resources.imported.assets,
                render: &self.resources.imported.render,
                tables: &self.resources.tables,
                biomes: &self.resources.biomes,
                visibility: self.resources.tables.visibility.clone(),
                zoom: self.zoom,
                blend_radius: 0,
                smooth_lighting: true,
                cutout_leaves: true,
            };
            let mesh = mesher.mesh(&region, sy);
            self.renderer.upload((x, sy, z), &mesh);
            self.sections += 1;
        }
        if self.teleported
            && !self.loaded
            && self.world.chunks.contains_key(&(
                (self.camera.x.floor() as i32) >> 4,
                (self.camera.z.floor() as i32) >> 4,
            ))
        {
            self.send(play::Serverbound::PlayerLoaded(Empty))?;
            self.loaded = true;
        }
        if self.last_tick.elapsed().as_millis() >= 50
            && matches!(
                self.session.phase,
                horizon_protocol::client_session::Phase::Play
            )
        {
            self.send(play::Serverbound::ClientTickEnd(Empty))?;
            self.last_tick = horizon_client_render::time::Instant::now();
        }
        let size = self.window.inner_size();
        let hud = HudBatch::new([size.width, size.height]);
        let mut frame = frame(&hud, self.frames);
        frame.camera = self.camera + DVec3::new(0., 1.62, 0.);
        frame.yaw = self.yaw;
        frame.pitch = self.pitch;
        frame.sky = self.environment.sample(self.world.day_time, None, None);
        frame.distance = 80.;
        frame.detail_distance = 80.;
        if self.renderer.render(&frame)? {
            self.frames += 1;
        }
        let body = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .body()
            .unwrap();
        body.set_attribute("data-horizon-world-peers", &self.peers.len().to_string())
            .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-world-chunks",
            &self.world.chunks.len().to_string(),
        )
        .map_err(|e| format!("{e:?}"))?;
        body.set_attribute("data-horizon-world-sections", &self.sections.to_string())
            .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-world-resident",
            &self.renderer.resident_sections().to_string(),
        )
        .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-world-drawn",
            &self.renderer.drawn_sections.to_string(),
        )
        .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}
