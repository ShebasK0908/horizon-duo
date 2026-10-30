//! Server chunk packets decoded into the client's existing chunk representation.
use crate::chunk::{BIOME_STRATEGY, Chunk, ColumnLayout, Light, Section, block_strategy};
use horizon_core::paletted::{PalettedContainer, Strategy};
use horizon_protocol::{
    chunk::{LightData, Paletted, SectionLight},
    packets::play,
    registry::DimensionType,
};
use std::sync::Arc;
/// What turning a chunk packet into a client column depends on: the
/// dimension's height and sky, and the server's biome registry mapped to
/// ours. The connection replaces it when either changes, so a column decoded
/// with an older one is decoded again.
#[derive(Default)]
pub struct ChunkFormat {
    /// The column decoded chunks hold: the client's for the level.
    column: ColumnLayout,
    min_y: i32,
    sections: usize,
    pub has_skylight: bool,
    biome_map: Vec<u16>,
    biome_bits: u8,
}

impl ChunkFormat {
    pub fn new(dimension: &DimensionType, biome_map: Vec<u16>, biome_bits: u8) -> Self {
        Self {
            column: ColumnLayout::VANILLA,
            min_y: dimension.min_y,
            sections: dimension.sections(),
            has_skylight: dimension.has_skylight,
            biome_map,
            biome_bits,
        }
    }
    /// Decodes into chunks of `column`.
    pub fn in_column(self, column: ColumnLayout) -> Self {
        Self { column, ..self }
    }
    /// The column decoded chunks hold.
    pub fn layout(&self) -> ColumnLayout {
        self.column
    }
    pub fn with_dimension(&self, dimension: &DimensionType) -> Self {
        Self {
            column: self.column,
            ..Self::new(dimension, self.biome_map.clone(), self.biome_bits)
        }
    }

    /// LevelChunkSection data into the client's column.
    pub fn column(&self, p: &play::LevelChunkWithLight) -> Result<Chunk, String> {
        let sections = p
            .chunk
            .sections(self.sections, 16, self.biome_bits)
            .map_err(|e| e.to_string())?;
        let mut chunk = Chunk::with_layout(p.x, p.z, self.column);
        let offset = (self.min_y - self.column.min_y()).div_euclid(16);
        let column = self.column.sections() as i32;
        for (i, s) in sections.iter().enumerate() {
            let target = offset + i as i32;
            if !(0..column).contains(&target) {
                continue;
            }
            // A global section is as wide as the server's registry: 16 bits
            // from a vanilla server, wider from a Horizon host with the same
            // content mods. Read it at that width, then widen it to ours.
            let ours = block_strategy();
            let strategy = if ours.wire_has_palette(s.states.bits) {
                ours
            } else {
                Strategy::blocks(s.states.bits)
            };
            let palette = s.states.palette.iter().map(|&p| p as u32).collect();
            let words = s.states.data.iter().map(|&w| w as u64).collect();
            let mut blocks = PalettedContainer::from_wire(strategy, s.states.bits, palette, words)
                .map_err(|e| e.to_string())?;
            if strategy.global_bits != ours.global_bits {
                let mut values = [0u32; 4096];
                blocks.unpack(&mut values);
                // Ids past our registry (a host with mods we lack) read as air.
                let limit = 1u64 << ours.global_bits;
                for v in &mut values {
                    if u64::from(*v) >= limit {
                        *v = 0;
                    }
                }
                blocks = PalettedContainer::from_values(ours, &values);
            }
            let mut section = Section {
                blocks,
                biomes: PalettedContainer::new(BIOME_STRATEGY, 0),
                light: Light::Uniform(0xF0),
            };
            section.compact();
            chunk.sections[target as usize] = Arc::new(section);
        }
        let biomes: Vec<_> = sections.iter().map(|s| &s.biomes).collect();
        self.put_biomes(&mut chunk, &biomes);
        // ClientPacketListener.applyLightData: the column keeps the light
        // the server sent; nothing is relit.
        let (layers, above) = self.light(&p.light);
        crate::light::light_from_layers(&mut chunk, &layers, above, self.has_skylight);
        Ok(chunk)
    }

    /// The dimension's sections' biome containers (in the server's
    /// registry) into the column's sections. Sections outside the
    /// dimension's range take its nearest biomes (the column's top or bottom
    /// cell layer repeated).
    fn put_biomes(&self, chunk: &mut Chunk, biomes: &[&Paletted]) {
        if biomes.is_empty() {
            return;
        }
        let offset = (self.min_y - self.column.min_y()).div_euclid(16);
        // Biome table index per quart cell of the dimension's own sections.
        let biome_cells: Vec<[u32; 64]> = biomes
            .iter()
            .map(|b| {
                std::array::from_fn(|j| {
                    u32::from(self.biome_map.get(b.get(j) as usize).copied().unwrap_or(0))
                })
            })
            .collect();
        let last = biome_cells.len() - 1;
        for si in 0..self.column.sections() as i32 {
            let i = si - offset;
            let cells = if (0..biome_cells.len() as i32).contains(&i) {
                biome_cells[i as usize]
            } else {
                let (edge, layer) = if i < 0 { (0, 0) } else { (last, 3) };
                std::array::from_fn(|j| biome_cells[edge][layer * 16 + j % 16])
            };
            let mut container = PalettedContainer::from_values(BIOME_STRATEGY, &cells);
            if let Some(v) = container.uniform() {
                container = PalettedContainer::new(BIOME_STRATEGY, v);
            }
            Arc::make_mut(&mut chunk.sections[si as usize]).biomes = container;
        }
    }

    /// `LevelChunk.replaceBiomes`: each of the dimension's sections reads
    /// its biome container from the packet's buffer, in order.
    pub fn replace_biomes(
        &self,
        chunk: &mut Chunk,
        data: &play::ChunkBiomeData,
    ) -> Result<(), String> {
        let biomes = data
            .biomes(self.sections, self.biome_bits)
            .map_err(|e| e.to_string())?;
        self.put_biomes(chunk, &biomes.iter().collect::<Vec<_>>());
        Ok(())
    }

    /// `ClientPacketListener.readSectionList`: the sky and block layers of a
    /// light packet, by section of the client's column, and the sky light
    /// above the column's top section (from the light section just above
    /// it, else open sky).
    pub fn light(&self, light: &LightData) -> (Vec<SectionLight>, [u8; 256]) {
        let column = self.column.sections() as i32;
        let mut layers: Vec<SectionLight> = vec![(None, None); self.column.sections()];
        let sky = self.has_skylight;
        let mut above = [if sky { 15 } else { 0 }; 256];
        // Light section b is the dimension's section b - 1.
        let first = (self.min_y - self.column.min_y()).div_euclid(16) - 1;
        for (b, layer) in light.layers(self.sections + 2).into_iter().enumerate() {
            let si = first + b as i32;
            if (0..column).contains(&si) {
                layers[si as usize] = layer;
            } else if si == column
                && sky
                && let Some(l) = &layer.0
            {
                for (i, a) in above.iter_mut().enumerate() {
                    *a = (l[i >> 1] >> ((i & 1) * 4)) & 15;
                }
            }
        }
        (layers, above)
    }
}
