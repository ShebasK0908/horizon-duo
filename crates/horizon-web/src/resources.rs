use horizon_assets::source::{ResourceBundle, ResourceSource};
use horizon_client_render::{
    Result,
    assets::{Assets, RenderAssets},
    biome::Biomes,
    blocks::BlockFacts,
    mesher::MeshTables,
    models,
};
use horizon_protocol::registry::Builtin;
use std::collections::BTreeMap;

pub struct ImportedResources {
    pub source: ResourceBundle,
    pub assets: Assets,
    pub render: RenderAssets,
}
impl ImportedResources {
    pub fn from_zip(bytes: Vec<u8>) -> Result<Self> {
        let zip = horizon_client_render::zip::Zip::from_bytes(bytes)?;
        let mut files = BTreeMap::new();
        let mut total = 0usize;
        for name in zip.names() {
            let bytes = zip.read(name)?;
            total = total
                .checked_add(bytes.len())
                .ok_or("Resource bundle too large")?;
            if total > 1024 * 1024 * 1024 {
                return Err("Resource bundle exceeds 1 GiB".into());
            }
            files.insert(name.to_owned(), bytes);
        }
        let source = ResourceBundle::new(files)?;
        let manifest: serde_json::Value =
            serde_json::from_slice(&source.read("horizon-resources.json")?)?;
        if manifest["schema"].as_i64() != Some(7)
            || manifest["minecraft"].as_str() != Some(horizon_core::VANILLA_VERSION)
            || manifest["data_version"].as_i64()
                != Some(i64::from(horizon_core::version::DATA_VERSION))
        {
            return Err(
                "Resource manifest does not match Horizon's prepared resource format".into(),
            );
        }
        let assets = Assets::load_from(&source)?;
        let render = RenderAssets::load_from(&source, &assets)?;
        Ok(Self {
            source,
            assets,
            render,
        })
    }
}
pub struct WorldResources {
    pub imported: ImportedResources,
    pub tables: MeshTables,
    pub facts: BlockFacts,
    pub player_model: horizon_client_render::entity_model::Part,
    pub player_skin: (u32, u32, Vec<u8>),
    pub biomes: Biomes,
    pub builtin: Builtin,
    pub identity: [u8; 32],
    pub motion: horizon_client_physics::motion::MotionResources,
    pub collision: horizon_world::block_collision::BlockCollisionRegistry<'static>,
}
impl WorldResources {
    pub fn new(imported: ImportedResources) -> Result<Self> {
        let source = &imported.source;
        let behaviours = serde_json::from_slice(&source.read("block-behaviours.json")?)?;
        let facts = BlockFacts::from_bytes(
            &source.read("gameplay-facts.json")?,
            imported.assets.registry,
            &behaviours,
        )?;
        let visibility = models::load_visibility_from(source, imported.assets.registry)?;
        let tables = MeshTables::new(&imported.assets, &imported.render, &facts, visibility)?;
        let models = horizon_client_render::entity_model::EntityModelDocument::from_bytes(
            &source.read("entity-models.json")?,
        )?;
        let player_model = models
            .model("minecraft:player#main")
            .ok_or("No prepared player model")?;
        let player_skin = horizon_client_render::hud::decode_bytes(
            &source.read("assets/minecraft/textures/entity/player/wide/steve.png")?,
        )?;
        let motion = horizon_client_physics::motion::MotionResources::load_from(
            source,
            &imported.assets,
            &facts,
        )?;
        let collision =
            horizon_client_render::collisions::load_from(source, imported.assets.registry)?;
        let biomes = Biomes::load_from(source)?;
        let builtin = Builtin::parse(std::str::from_utf8(&source.read("registries.json")?)?)?;
        let identity =
            horizon_protocol::registry::block_state_identity(&source.read("facts.json")?)
                .map_err(std::io::Error::other)?;
        Ok(Self {
            imported,
            tables,
            facts,
            player_model,
            player_skin,
            biomes,
            builtin,
            identity,
            motion,
            collision,
        })
    }
}
