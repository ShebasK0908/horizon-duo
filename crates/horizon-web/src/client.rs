use crate::{
    render_frame::frame,
    resources::{ImportedResources, WorldResources},
    socket::ClientConnection,
};
use glam::DVec3;
use horizon_client_physics::{
    motion::{MotionWorld, PlayerMotion},
    sprint::Keys,
};
use horizon_client_render::picking::{self, Hit};
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
use horizon_core::geometry::Vec3;
use horizon_protocol::player_sync::{PositionSender, teleport_velocity};
use horizon_protocol::{
    client_session::{ClientSession, Event as SessionEvent},
    packets::{Empty, configuration as cfg, play},
    registry::{Dynamic, ceil_log2},
};
use horizon_world::{
    registry::BlockStateId,
    stepped_interpolation::{Path, PosRot, SteppedInterpolation},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};
use wasm_bindgen::{JsCast, prelude::*};
use winit::{
    event::{DeviceEvent, ElementState, Event, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    platform::web::{EventLoopExtWebSys, WindowExtWebSys},
    window::Window,
};

struct Peer {
    network: [f64; 3],
    current: PosRot,
    interpolation: SteppedInterpolation,
}
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
    player: Option<PlayerMotion>,
    keys: HashSet<KeyCode>,
    position_sender: PositionSender,
    abilities: Option<play::PlayerAbilities>,
    game_type: i32,
    entity_id: i32,
    dirty: VecDeque<SectionKey>,
    pending: HashSet<SectionKey>,
    frames: u64,
    sections: u64,
    peers: HashMap<i32, Peer>,
    sequence: i32,
    selected_item: Option<String>,
    observed_blocks: VecDeque<([i32; 3], u32)>,
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
    document
        .get_element_by_id("viewport")
        .unwrap()
        .append_child(&canvas)?;
    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(960, 640));
    let render = &resources.imported.render;
    let select = document.get_element_by_id("block").unwrap();
    if let Some(items) = resources.builtin.get("minecraft:item") {
        let mut names: Vec<_> = (0..items.names.len())
            .filter_map(|id| items.name(id as i32))
            .filter(|name| {
                resources
                    .imported
                    .assets
                    .registry
                    .default_state(name)
                    .is_some()
            })
            .collect();
        names.sort_unstable();
        for name in names {
            let option = document.create_element("option")?;
            option.set_attribute("value", name)?;
            option.set_text_content(Some(name.strip_prefix("minecraft:").unwrap_or(name)));
            select.append_child(&option)?;
        }
        select
            .clone()
            .dyn_into::<web_sys::HtmlSelectElement>()?
            .set_value("minecraft:stone");
    }
    for id in ["materials", "controls", "crosshair"] {
        document
            .get_element_by_id(id)
            .unwrap()
            .remove_attribute("hidden")?;
    }
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
        (
            &resources.player_skin.2,
            resources.player_skin.0,
            resources.player_skin.1,
        ),
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
        player: None,
        keys: HashSet::new(),
        position_sender: PositionSender::default(),
        abilities: None,
        game_type: 0,
        entity_id: 0,
        dirty: VecDeque::new(),
        pending: HashSet::new(),
        frames: 0,
        sections: 0,
        peers: HashMap::new(),
        sequence: 0,
        selected_item: None,
        observed_blocks: VecDeque::new(),
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
            event: WindowEvent::KeyboardInput { event, .. },
            ..
        } => {
            if let PhysicalKey::Code(key) = event.physical_key {
                if event.state == ElementState::Pressed {
                    client.keys.insert(key);
                    if !event.repeat
                        && matches!(key, KeyCode::KeyF | KeyCode::KeyR)
                        && let Err(error) = client.interact(key == KeyCode::KeyF)
                    {
                        show_error(&error.to_string());
                    }
                } else {
                    client.keys.remove(&key);
                }
            }
        }
        Event::WindowEvent {
            event: WindowEvent::Focused(false),
            ..
        } => {
            client.keys.clear();
        }
        Event::WindowEvent {
            event:
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button,
                    ..
                },
            ..
        } => {
            if button == MouseButton::Right {
                if let Err(error) = client.interact(true) {
                    show_error(&error.to_string());
                }
            } else if button == MouseButton::Left {
                if web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .pointer_lock_element()
                    .is_some()
                {
                    if let Err(error) = client.interact(false) {
                        show_error(&error.to_string());
                    }
                } else {
                    client.request_pointer_lock();
                }
            }
        }
        Event::DeviceEvent {
            event: DeviceEvent::MouseMotion { delta },
            ..
        } => {
            client.yaw += delta.0 as f32 * 0.15;
            client.pitch = (client.pitch + delta.1 as f32 * 0.15).clamp(-90., 90.);
        }
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
    fn request_pointer_lock(&self) {
        let Some(canvas) = self.window.canvas() else {
            return;
        };
        let result = js_sys::Reflect::get(&canvas, &JsValue::from_str("requestPointerLock"))
            .and_then(|value| value.dyn_into::<js_sys::Function>())
            .and_then(|function| function.call0(&canvas));
        if let Ok(value) = result
            && let Ok(promise) = value.dyn_into::<js_sys::Promise>()
        {
            wasm_bindgen_futures::spawn_local(async move {
                if wasm_bindgen_futures::JsFuture::from(promise).await.is_err() {
                    log::info!("Pointer lock unavailable; arrow keys still control the view");
                }
            });
        }
    }
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
                    self.peers.insert(
                        packet.id,
                        Peer {
                            network: packet.position,
                            current: PosRot {
                                position: packet.position,
                                yaw: horizon_protocol::buf::unpack_degrees(packet.yaw),
                                pitch: horizon_protocol::buf::unpack_degrees(packet.pitch),
                            },
                            interpolation: SteppedInterpolation::new(3),
                        },
                    );
                }
            }
            C::MoveEntityPos(play::MoveEntityPos(m))
            | C::MoveEntityPosRot(play::MoveEntityPosRot(m))
            | C::MoveEntityRot(play::MoveEntityRot(m)) => {
                if let Some(peer) = self.peers.get_mut(&m.id) {
                    let path = m.delta.as_ref().map(|d| match d {
                        play::VecDelta::Linear(_) => Path::Linear(d.end(peer.network)),
                        play::VecDelta::Stepped(steps) => {
                            let mut at = peer.network;
                            Path::Stepped(
                                steps
                                    .iter()
                                    .map(|(delta, ticks)| {
                                        at = play::VecDelta::apply(at, *delta);
                                        (at, *ticks)
                                    })
                                    .collect(),
                            )
                        }
                    });
                    if let Some(path) = &path {
                        peer.network = path.end();
                    }
                    peer.interpolation.interpolate_to(
                        peer.current,
                        path,
                        m.rotation.map(|(yaw, pitch)| {
                            (
                                horizon_protocol::buf::unpack_degrees(yaw),
                                horizon_protocol::buf::unpack_degrees(pitch),
                            )
                        }),
                    );
                }
            }
            C::EntityPositionSync(p) => {
                if let Some(peer) = self.peers.get_mut(&p.id) {
                    let path = match p.path {
                        play::PositionPath::Linear(at) => Path::Linear(at),
                        play::PositionPath::Stepped(steps) => Path::Stepped(steps),
                    };
                    peer.network = path.end();
                    peer.interpolation.interpolate_to(
                        peer.current,
                        Some(path),
                        Some((p.yaw, p.pitch)),
                    );
                }
            }
            C::TeleportEntity(p) => {
                if let Some(peer) = self.peers.get_mut(&p.id) {
                    let mut position = p.position;
                    for (i, bit) in [play::relative::X, play::relative::Y, play::relative::Z]
                        .into_iter()
                        .enumerate()
                    {
                        if p.relatives & bit != 0 {
                            position[i] += peer.network[i];
                        }
                    }
                    let yaw = p.yaw
                        + if p.relatives & play::relative::Y_ROT != 0 {
                            peer.current.yaw
                        } else {
                            0.
                        };
                    let pitch = p.pitch
                        + if p.relatives & play::relative::X_ROT != 0 {
                            peer.current.pitch
                        } else {
                            0.
                        };
                    peer.network = position;
                    peer.current = PosRot {
                        position,
                        yaw,
                        pitch,
                    };
                    peer.interpolation = SteppedInterpolation::new(3);
                }
            }
            C::RemoveEntities(packet) => {
                for id in packet.ids {
                    self.peers.remove(&id);
                }
            }
            C::Login(packet) => {
                self.entity_id = packet.player_id;
                self.dimension(&packet.spawn)?;
            }
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
                let velocity = self.player.as_ref().map_or(Vec3::ZERO, |p| p.velocity);
                let mut player = match self.player.take() {
                    Some(player) => player,
                    None => PlayerMotion::new(
                        &self.resources.motion,
                        Vec3::new(self.camera.x, self.camera.y, self.camera.z),
                    )?,
                };
                player.position = Vec3::new(self.camera.x, self.camera.y, self.camera.z);
                player.velocity = teleport_velocity(
                    velocity,
                    (player.yaw, player.pitch),
                    (self.yaw, self.pitch),
                    &packet,
                );
                player.client_side = true;
                player.spectator = self.game_type == 3;
                self.player = Some(player);
                self.apply_abilities();
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
            C::PlayerAbilities(packet) => {
                self.abilities = Some(packet);
                self.apply_abilities();
            }
            C::SetHealth(packet) => {
                if let Some(player) = &mut self.player {
                    player.set_health(packet.health, false)?;
                    player.health.food.food = packet.food;
                    player.health.food.saturation = packet.saturation;
                }
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
            C::BlockUpdate(packet) => self.server_block(packet.pos, packet.state)?,
            C::SectionBlocksUpdate(packet) => {
                for (pos, state) in packet.blocks() {
                    self.server_block(pos, state)?;
                }
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
        self.game_type = spawn.game_type;
        self.player = None;
        self.peers.clear();
        self.teleported = false;
        self.position_sender = PositionSender::default();
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
    fn apply_abilities(&mut self) {
        if let (Some(player), Some(abilities)) = (&mut self.player, &self.abilities) {
            player.invulnerable = abilities.flags & 1 != 0;
            player.flying = abilities.flags & 2 != 0;
            player.may_fly = abilities.flags & 4 != 0;
            player.flying_speed = abilities.flying_speed;
        }
    }
    fn tick_player(&mut self) -> Result<()> {
        for peer in self.peers.values_mut() {
            if let Some(current) = peer.interpolation.interpolate() {
                peer.current = current;
            }
        }
        if !self.loaded {
            return Ok(());
        }
        if self.keys.contains(&KeyCode::ArrowLeft) {
            self.yaw -= 4.;
        }
        if self.keys.contains(&KeyCode::ArrowRight) {
            self.yaw += 4.;
        }
        if self.keys.contains(&KeyCode::ArrowUp) {
            self.pitch = (self.pitch - 4.).max(-90.);
        }
        if self.keys.contains(&KeyCode::ArrowDown) {
            self.pitch = (self.pitch + 4.).min(90.);
        }
        let held = |key| self.keys.contains(&key);
        let keys = Keys {
            forward: held(KeyCode::KeyW),
            backward: held(KeyCode::KeyS),
            left: held(KeyCode::KeyA),
            right: held(KeyCode::KeyD),
            jump: held(KeyCode::Space),
            shift: held(KeyCode::ShiftLeft),
            sprint: held(KeyCode::ControlLeft),
            analog: None,
        };
        let input = u8::from(keys.forward)
            | (u8::from(keys.backward) << 1)
            | (u8::from(keys.left) << 2)
            | (u8::from(keys.right) << 3)
            | (u8::from(keys.jump) << 4)
            | (u8::from(keys.shift) << 5)
            | (u8::from(keys.sprint) << 6);
        let Some(player) = &mut self.player else {
            return Ok(());
        };
        player.yaw = self.yaw;
        player.pitch = self.pitch;
        let was_flying = player.flying;
        let was_sprinting = player.sprinting;
        MotionWorld {
            entities: &[],
            player,
            scene: &mut self.world,
            assets: &self.resources.imported.assets,
            collision_binding: &self.resources.collision,
            resources: &self.resources.motion,
        }
        .controlled_simulation_tick(keys)?;
        player.events.clear();
        self.camera = DVec3::new(player.position.x, player.position.y, player.position.z);
        let flying = player.flying;
        let sprinting = player.sprinting;
        let position = self.position_sender.update(
            [self.camera.x, self.camera.y, self.camera.z],
            (self.yaw, self.pitch),
            player.ground.on_ground,
            player.collision.horizontal,
        );
        self.send(play::Serverbound::PlayerInput(play::PlayerInput {
            flags: input,
        }))?;
        if flying != was_flying {
            self.send(play::Serverbound::PlayerAbilities(
                play::PlayerAbilitiesRequest {
                    flags: if flying { 2 } else { 0 },
                },
            ))?;
        }
        if sprinting != was_sprinting {
            self.send(play::Serverbound::PlayerCommand(play::PlayerCommand {
                id: self.entity_id,
                action: if sprinting { 1 } else { 2 },
                data: 0,
            }))?;
        }
        if let Some(packet) = position {
            self.send(packet)?;
        }
        Ok(())
    }
    fn server_block(&mut self, pos: [i32; 3], state: i32) -> Result<()> {
        self.world
            .set(pos[0], pos[1], pos[2], BlockStateId(state as u32))?;
        self.world.light_edits.clear();
        self.world.dirty_light.clear();
        self.observed_blocks.push_back((pos, state as u32));
        if self.observed_blocks.len() > 64 {
            self.observed_blocks.pop_front();
        }
        let layout = self.world.layout();
        let mut changed = Vec::new();
        for dz in -1..=1 {
            for dx in -1..=1 {
                let (x, z) = ((pos[0] >> 4) + dx, (pos[2] >> 4) + dz);
                if !self.world.chunks.contains_key(&(x, z)) {
                    continue;
                }
                for dy in -1..=1 {
                    if let Some(sy) = layout.section_index(pos[1] + dy * 16) {
                        changed.push((x, sy as i32, z));
                    }
                }
            }
        }
        self.dirty.retain(|key| !changed.contains(key));
        for key in changed {
            self.pending.insert(key);
            self.dirty.push_front(key);
        }
        Ok(())
    }
    fn eye(&self) -> DVec3 {
        self.camera
            + DVec3::new(
                0.,
                self.player
                    .as_ref()
                    .map_or(1.62, |p| f64::from(p.dimensions.eye_height)),
                0.,
            )
    }
    fn pick(&self) -> Option<Hit> {
        picking::clip_ray(
            self.eye().to_array(),
            horizon_client_render::camera::view_vector(self.yaw, self.pitch),
            if self.game_type == 1 { 5. } else { 4.5 },
            &self.world,
            &self.resources.facts,
            |pos| {
                let state = self.world.block(pos[0], pos[1], pos[2]);
                let facts = self.resources.facts.get(state);
                let offset = if facts.outline_offset {
                    self.resources
                        .tables
                        .offset(state, pos[0], pos[2])
                        .unwrap_or([0.; 3])
                } else {
                    [0.; 3]
                };
                facts
                    .outline
                    .iter()
                    .map(|b| std::array::from_fn(|i| f64::from(pos[i % 3]) + offset[i % 3] + b[i]))
                    .collect()
            },
        )
    }
    fn interact(&mut self, place: bool) -> Result<()> {
        if !self.loaded {
            return Ok(());
        }
        let Some(hit) = self.pick() else {
            return Ok(());
        };
        self.sequence = self.sequence.wrapping_add(1);
        let face = [
            [0, -1, 0],
            [0, 1, 0],
            [0, 0, -1],
            [0, 0, 1],
            [-1, 0, 0],
            [1, 0, 0],
        ]
        .iter()
        .position(|f| *f == hit.face)
        .unwrap_or(1) as i32;
        if place {
            if self.game_type == 1 {
                let name = web_sys::window()
                    .unwrap()
                    .document()
                    .unwrap()
                    .get_element_by_id("block")
                    .unwrap()
                    .dyn_into::<web_sys::HtmlSelectElement>()
                    .unwrap()
                    .value();
                if self.selected_item.as_ref() != Some(&name) {
                    let item = self
                        .resources
                        .builtin
                        .get("minecraft:item")
                        .and_then(|items| items.id(&name))
                        .ok_or("Unknown creative block")?;
                    self.send(play::Serverbound::SetCreativeModeSlot(
                        play::SetCreativeModeSlot {
                            slot: 36,
                            item: Some(horizon_protocol::item::Stack::new(item, 64)),
                        },
                    ))?;
                    self.send(play::Serverbound::SetCarriedItem(play::SetCarriedItem {
                        slot: 0,
                    }))?;
                    self.selected_item = Some(name);
                }
            }
            self.send(play::Serverbound::UseItemOn(play::UseItemOn {
                hand: 0,
                pos: hit.pos,
                face,
                cursor: std::array::from_fn(|i| (hit.point[i] - f64::from(hit.pos[i])) as f32),
                inside: false,
                world_border: false,
                sequence: self.sequence,
            }))?;
        } else {
            self.send(play::Serverbound::PlayerAction(play::PlayerAction {
                action: 0,
                pos: hit.pos,
                direction: face as u8,
                sequence: self.sequence,
            }))?;
        }
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
            self.tick_player()?;
            self.send(play::Serverbound::ClientTickEnd(Empty))?;
            self.last_tick = horizon_client_render::time::Instant::now();
        }
        let size = self.window.inner_size();
        let hud = HudBatch::new([size.width, size.height]);
        let mut frame = frame(&hud, self.frames);
        frame.camera = self.eye();
        let mut entities = Vec::new();
        for peer in self.peers.values() {
            let relative = DVec3::from_array(peer.current.position) - frame.camera;
            let base = glam::Mat4::from_translation(relative.as_vec3())
                * glam::Mat4::from_rotation_y((180. - peer.current.yaw).to_radians())
                * glam::Mat4::from_scale(glam::Vec3::new(-1., -1., 1.))
                * glam::Mat4::from_translation(glam::Vec3::new(0., -1.501, 0.));
            horizon_client_render::entity_model::render_model(
                &self.resources.player_model,
                base,
                &|part| part.pose,
                [0., 0., 1., 1.],
                0x00f000f0,
                0,
                &mut entities,
            );
        }
        frame.entities = &entities;
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
        body.set_attribute("data-horizon-mesh-pending", &self.pending.len().to_string())
            .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-target",
            &serde_json::to_string(&self.pick().map(|h| {
                (
                    h.pos,
                    h.face,
                    self.world.block(h.pos[0], h.pos[1], h.pos[2]).0,
                )
            }))?,
        )
        .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-observed-blocks",
            &serde_json::to_string(&self.observed_blocks)?,
        )
        .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-peer-positions",
            &serde_json::to_string(
                &self
                    .peers
                    .iter()
                    .map(|(id, p)| (*id, p.network))
                    .collect::<Vec<_>>(),
            )?,
        )
        .map_err(|e| format!("{e:?}"))?;
        body.set_attribute("data-horizon-peer-vertices", &entities.len().to_string())
            .map_err(|e| format!("{e:?}"))?;
        body.set_attribute(
            "data-horizon-position",
            &format!(
                "{:.4},{:.4},{:.4}",
                self.camera.x, self.camera.y, self.camera.z
            ),
        )
        .map_err(|e| format!("{e:?}"))?;
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
