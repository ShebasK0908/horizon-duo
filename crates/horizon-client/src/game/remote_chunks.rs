//! A server's chunk columns (`ClientboundLevelChunkWithLightPacket`): the
//! sections and light of each packet become a client column on a decoder
//! thread as soon as the packet arrives, so the frame that handles it in
//! order only inserts the result (`ClientPacketListener.handleLevelChunkWithLight`).
use super::Game;
use horizon_client::{chunk::Chunk, streaming::Streaming};
use horizon_protocol::packets::play::{self, Clientbound as C};
use std::sync::{
    Arc,
    mpsc::{self, TryRecvError},
};

pub(super) use horizon_client::remote_chunks::ChunkFormat;

type Decoded = (Box<C>, Result<Chunk, String>);

/// One thread that decodes chunk packets in the order they arrived; its
/// results come back in that order, one per `submit`.
pub(super) struct ChunkDecoder {
    jobs: mpsc::Sender<(Arc<ChunkFormat>, Box<C>)>,
    done: mpsc::Receiver<Decoded>,
}

impl ChunkDecoder {
    pub(super) fn new() -> Self {
        let (jobs, todo) = mpsc::channel::<(Arc<ChunkFormat>, Box<C>)>();
        let (finished, done) = mpsc::channel();
        // The thread ends with the connection, when `jobs` is dropped.
        let spawned = std::thread::Builder::new()
            .name("Chunk decoder".into())
            .spawn(move || {
                // A frame waits for its columns, as it would for the render
                // thread's own work.
                horizon_threads::favour_current();
                for (format, packet) in todo {
                    let column = match &*packet {
                        C::LevelChunkWithLight(p) => format.column(p),
                        _ => Err("not a chunk packet".into()),
                    };
                    if finished.send((packet, column)).is_err() {
                        break;
                    }
                }
            });
        if let Err(e) = spawned {
            log::warn!("Horizon: no chunk decoder thread ({e}); chunks decode in the frame");
        }
        Self { jobs, done }
    }

    /// Start decoding a chunk packet with the format it arrived under.
    /// Without a decoder thread the packet comes back undecoded.
    pub(super) fn submit(&self, format: Arc<ChunkFormat>, packet: Box<C>) -> Option<Box<C>> {
        self.jobs.send((format, packet)).err().map(|e| (e.0).1)
    }

    /// The oldest submitted packet and its column, unless it is still
    /// decoding: a frame never waits for the decoder.
    pub(super) fn next(&self) -> Next {
        match self.done.try_recv() {
            Ok(decoded) => Next::Ready(decoded),
            Err(TryRecvError::Empty) => Next::Decoding,
            // The decoder thread panicked: its columns are gone.
            Err(TryRecvError::Disconnected) => Next::Lost,
        }
    }
}

pub(super) enum Next {
    Ready(Decoded),
    Decoding,
    Lost,
}

impl Game<'_> {
    /// `handleLevelChunkWithLight`: the column replaces any loaded one, and
    /// its block entities load their update tags. `decoded` is the column the
    /// decoder made with `format`; another format since means decode again.
    pub(super) fn remote_chunk(
        &mut self,
        p: &play::LevelChunkWithLight,
        decoded: Option<(&Arc<ChunkFormat>, Result<Chunk, String>)>,
    ) {
        // ClientChunkCache.replaceWithPacketData: a column outside the
        // storage around the view centre is ignored.
        if !self.remote_chunk_in_range(p.x, p.z) {
            log::warn!(
                "Ignoring chunk since it's not in the view range: {}, {}",
                p.x,
                p.z
            );
            return;
        }
        let Some(format) = self.remote.as_ref().map(|r| r.chunk_format.clone()) else {
            return;
        };
        let column = match decoded {
            Some((used, column)) if Arc::ptr_eq(used, &format) => column,
            _ => format.column(p),
        };
        let chunk = match column {
            Ok(chunk) => chunk,
            Err(e) => {
                log::warn!("Horizon: chunk {} {}: {e}", p.x, p.z);
                return;
            }
        };
        let (x, z) = (p.x, p.z);
        self.streaming.insert_remote(&mut self.world, chunk);
        self.world
            .block_entities
            .retain(|k, _| (k.0 >> 4, k.2 >> 4) != (x, z));
        for info in &p.chunk.block_entities {
            let pos = (
                x * 16 + i32::from(info.packed_xz as u8 >> 4),
                i32::from(info.y),
                z * 16 + i32::from(info.packed_xz as u8 & 15),
            );
            let state = self.world.block(pos.0, pos.1, pos.2);
            self.sync_block_entity(pos, state);
            // LevelChunk.replaceWithPacketData: the block entity loads its update tag.
            self.remote_block_entity_data([pos.0, pos.1, pos.2], info.kind, &info.tag);
        }
    }

    /// `ClientPacketListener.handleChunksBiomes`: each loaded column takes
    /// its new biomes (`ClientChunkCache.replaceBiomes`), then every section
    /// of it and of its neighbours is drawn again, as their colours blend
    /// across columns (`LevelExtractor.setSectionDirty`).
    pub(super) fn remote_chunks_biomes(&mut self, p: &play::ChunksBiomes) {
        let Some(format) = self.remote.as_ref().map(|r| r.chunk_format.clone()) else {
            return;
        };
        for data in &p.chunks {
            let [x, z] = data.pos;
            if !self.remote_chunk_in_range(x, z) {
                log::warn!("Ignoring chunk since it's not in the view range: {x}, {z}");
                continue;
            }
            let Some(chunk) = self.world.chunks.get_mut(&(x, z)) else {
                log::warn!("Ignoring chunk since it's not present: {x}, {z}");
                continue;
            };
            if let Err(e) = format.replace_biomes(Arc::make_mut(chunk), data) {
                log::warn!("Horizon: biomes of chunk {x} {z}: {e}");
            }
        }
        for data in &p.chunks {
            let [x, z] = data.pos;
            for dz in -1..=1 {
                for dx in -1..=1 {
                    if !self.world.chunks.contains_key(&(x + dx, z + dz)) {
                        continue;
                    }
                    for sy in 0..self.world.layout().sections() as i32 {
                        self.world.dirty_sections.insert((x + dx, sy, z + dz));
                    }
                }
            }
        }
    }

    /// `handleLightUpdatePacket`: the sections it names take its light.
    pub(super) fn remote_light_update(&mut self, p: &play::LightUpdate) {
        let Some(format) = self.remote.as_ref().map(|r| r.chunk_format.clone()) else {
            return;
        };
        if !self.world.chunks.contains_key(&(p.x, p.z)) {
            return;
        }
        let (layers, _) = format.light(&p.light);
        let Some(chunk) = self.world.chunks.get_mut(&(p.x, p.z)) else {
            return;
        };
        let changed = horizon_client::light::update_light_layers(
            Arc::make_mut(chunk),
            &layers,
            format.has_skylight,
        );
        Streaming::light_updated(&mut self.world, p.x, p.z, &changed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_protocol::{
        chunk::{ChunkData, LightData, Paletted, Section as Wire, Strategy as Wiring},
        registry::DimensionType,
    };

    /// A column of `state` with biome 1 everywhere, unlit.
    fn packet(x: i32, state: i32) -> Box<C> {
        let section = Wire {
            non_empty: 4096,
            fluids: 0,
            states: Paletted::from_values(&[state; 4096], Wiring::Blocks, 16),
            biomes: Paletted::from_values(&[1; 64], Wiring::Biomes, 2),
        };
        let none = LightData {
            sky_mask: Vec::new(),
            block_mask: Vec::new(),
            empty_sky_mask: Vec::new(),
            empty_block_mask: Vec::new(),
            sky: Vec::new(),
            block: Vec::new(),
        };
        Box::new(C::LevelChunkWithLight(play::LevelChunkWithLight {
            x,
            z: -3,
            chunk: ChunkData {
                heightmaps: Vec::new(),
                buffer: ChunkData::from_sections(&vec![section; 24]),
                block_entities: Vec::new(),
            },
            light: none,
        }))
    }

    /// Columns come back one per packet, in the order the packets went in,
    /// as the frame would have decoded them itself.
    #[test]
    fn the_decoder_returns_columns_in_order() {
        let overworld = DimensionType::vanilla("minecraft:overworld");
        let format = Arc::new(ChunkFormat::new(&overworld, vec![0, 7, 9], 2));
        let decoder = ChunkDecoder::new();
        for x in 0..20 {
            assert!(decoder.submit(format.clone(), packet(x, 1 + x)).is_none());
        }
        for x in 0..20 {
            let (packet, column) = loop {
                match decoder.next() {
                    Next::Ready(decoded) => break decoded,
                    Next::Decoding => std::thread::yield_now(),
                    Next::Lost => panic!("the decoder stopped"),
                }
            };
            let C::LevelChunkWithLight(p) = *packet else {
                panic!("not a chunk packet");
            };
            let (column, direct) = (column.unwrap(), format.column(&p).unwrap());
            assert_eq!((column.x, column.z), (x, -3));
            for (cx, y, cz) in [(0, -64, 0), (5, 70, 9), (15, 319, 15)] {
                assert_eq!(column.get(cx, y, cz), 1 + x as u32);
                assert_eq!(column.get(cx, y, cz), direct.get(cx, y, cz));
                // Wire biome 1 is our biome 7.
                assert_eq!(column.biome(cx, y, cz), 7);
            }
        }
        assert!(matches!(decoder.next(), Next::Decoding));
    }

    /// `LevelChunk.replaceBiomes`: the packet's containers go to the
    /// dimension's sections (y 0 to 255 in the Nether), mapped to our biome
    /// table; the blocks stay, and the sections outside the dimension take
    /// its nearest biomes as a chunk packet's do.
    #[test]
    fn chunks_biomes_replace_a_columns_biomes() {
        let nether = DimensionType::vanilla("minecraft:the_nether");
        let format = ChunkFormat::new(&nether, vec![0, 7, 9], 2);
        let mut chunk = Chunk::empty(2, -3);
        chunk.set(3, 70, 4, 5);
        // Wire biome 1 everywhere, with 2 in the lowest and the highest cells.
        let containers: Vec<Paletted> = (0..nether.sections())
            .map(|i| {
                let mut cells = [1; 64];
                if i == 0 {
                    cells[0] = 2;
                }
                if i == nether.sections() - 1 {
                    cells[63] = 2;
                }
                Paletted::from_values(&cells, Wiring::Biomes, 2)
            })
            .collect();
        let data = play::ChunkBiomeData::from_biomes([2, -3], &containers);
        format.replace_biomes(&mut chunk, &data).unwrap();
        assert_eq!(chunk.get(3, 70, 4), 5);
        assert_eq!(chunk.biome(8, 70, 8), 7);
        assert_eq!(chunk.biome(0, 0, 0), 9);
        assert_eq!(chunk.biome(4, 0, 0), 7);
        assert_eq!(chunk.biome(15, 255, 15), 9);
        // Below and above the Nether: its bottom and top cell layers.
        assert_eq!(chunk.biome(0, -64, 0), 9);
        assert_eq!(chunk.biome(15, 319, 15), 9);
        assert_eq!(chunk.biome(0, 319, 0), 7);
        // A buffer short of the dimension's sections changes nothing.
        let short = play::ChunkBiomeData::from_biomes([2, -3], &containers[..3]);
        let mut other = Chunk::empty(2, -3);
        assert!(format.replace_biomes(&mut other, &short).is_err());
        assert_eq!(other.biome(8, 70, 8), 0);
    }
}
