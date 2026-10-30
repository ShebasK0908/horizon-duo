//! Parsed resource models for one startup or resource-pack reload.
use crate::Result;
use horizon_assets::source::{NativeResources, ResourceSource};
use horizon_assets::{model::CuboidModel, resolve::ModelLibrary};
use std::{collections::BTreeMap, path::Path, sync::Arc};

#[derive(Default)]
pub struct ParsedModels {
    models: BTreeMap<String, Arc<CuboidModel>>,
}
impl ParsedModels {
    /// Every `models/**.json` of a namespace directory, by identifier.
    pub fn load(root: &Path) -> Self {
        Self::load_from(&NativeResources(root))
    }
    pub fn load_from(source: &dyn ResourceSource) -> Self {
        let files: Vec<_> = source
            .files("assets")
            .unwrap_or_default()
            .into_iter()
            .filter_map(|path| {
                let relative = path.strip_prefix("assets/")?;
                let (namespace, relative) = relative.split_once("/models/")?;
                let name = relative.strip_suffix(".json")?;
                Some((format!("{namespace}:{name}"), path))
            })
            .collect();
        let parsed = crate::par::par_map(&files, |(_, path)| {
            let bytes = source.read(path).ok()?;
            CuboidModel::from_json(std::str::from_utf8(&bytes).ok()?)
                .ok()
                .map(Arc::new)
        });
        let models = files
            .into_iter()
            .zip(parsed)
            .filter_map(|((id, _), model)| Some((id, model?)))
            .collect();
        Self { models }
    }

    /// Blocks require every referenced resource, including parents. Items instead
    /// skip broken resources and use the resolver's missing-model fallback.
    pub fn block_library(&self, root: &Path, pending: Vec<String>) -> Result<ModelLibrary> {
        self.block_models(&NativeResources(root), pending, false)
    }
    /// [`Self::block_library`], but a model that does not read takes the
    /// built-in missing model, logged, as vanilla's model discovery does
    /// ("Missing block model"). Mods' content gets this; vanilla's resources
    /// always read.
    pub fn block_library_or_missing(
        &self,
        root: &Path,
        pending: Vec<String>,
    ) -> Result<ModelLibrary> {
        self.block_models(&NativeResources(root), pending, true)
    }
    pub fn block_library_from(
        &self,
        source: &dyn ResourceSource,
        pending: Vec<String>,
    ) -> Result<ModelLibrary> {
        self.block_models(source, pending, false)
    }
    pub fn block_library_or_missing_from(
        &self,
        source: &dyn ResourceSource,
        pending: Vec<String>,
    ) -> Result<ModelLibrary> {
        self.block_models(source, pending, true)
    }
    fn block_models(
        &self,
        source: &dyn ResourceSource,
        mut pending: Vec<String>,
        or_missing: bool,
    ) -> Result<ModelLibrary> {
        let mut models = BTreeMap::new();
        let mut skipped = std::collections::BTreeSet::new();
        while let Some(name) = pending.pop() {
            if models.contains_key(&name) || skipped.contains(&name) {
                continue;
            }
            let model = match self.models.get(&name) {
                Some(model) => Arc::clone(model),
                None => {
                    let read = || -> Result<CuboidModel> {
                        let (namespace, path) =
                            name.split_once(':').ok_or("invalid model identifier")?;
                        let bytes =
                            source.read(&format!("assets/{namespace}/models/{path}.json"))?;
                        Ok(CuboidModel::from_json(std::str::from_utf8(&bytes)?)?)
                    };
                    match read() {
                        Ok(model) => Arc::new(model),
                        Err(e) if or_missing => {
                            log::warn!("Horizon: missing block model {name}: {e}");
                            skipped.insert(name);
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
            };
            if let Some(parent) = &model.parent {
                pending.push(parent.clone());
            }
            models.insert(name, model);
        }
        Ok(ModelLibrary::from_shared(models)?)
    }
    pub fn into_item_library(self) -> std::result::Result<Arc<ModelLibrary>, String> {
        ModelLibrary::with_shared_item_generator(self.models)
            .map(Arc::new)
            .map_err(|e| format!("item models: {e:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use horizon_assets::resolve::{GENERATED_MODEL, MISSING_MODEL};
    use std::fs;

    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn write(&self, name: &str, json: &str) {
            let (namespace, path) = name.split_once(':').unwrap();
            let path = self
                .0
                .join(format!("assets/{namespace}/models/{path}.json"));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, json).unwrap();
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn shared_views_preserve_resolution_errors_builtins_and_reload() -> Result<()> {
        let fixture = Fixture(
            std::env::temp_dir().join(format!("horizon-shared-models-{}", std::process::id())),
        );
        fixture.write("horizon:parent", r##"{"textures":{"side":"#all","all":"block/stone"},"elements":[{"from":[0,0,0],"to":[16,8,16],"faces":{"up":{"texture":"#side"}}}]}"##);
        fixture.write("horizon:nested/child", r#"{"parent":"horizon:parent","textures":{"all":"block/glass"},"display":{"gui":{"rotation":[30,45,0]}}}"#);
        fixture.write("horizon:broken", "{");
        fixture.write("horizon:dangling", r#"{"parent":"horizon:absent"}"#);
        fixture.write("horizon:a", r#"{"parent":"horizon:b"}"#);
        fixture.write("horizon:b", r#"{"parent":"horizon:a"}"#);
        fixture.write(
            GENERATED_MODEL,
            r#"{"elements":[{"from":[0,0,0],"to":[1,1,1],"faces":{"up":{"texture":"particle"}}}],"gui_light":"side"}"#,
        );
        fixture.write(MISSING_MODEL, r#"{"elements":[]}"#);
        let parsed = ParsedModels::load(&fixture.0);
        let direct = ParsedModels::default();
        let names = vec![
            "horizon:nested/child".into(),
            GENERATED_MODEL.into(),
            MISSING_MODEL.into(),
            "horizon:a".into(),
        ];
        let old = direct.block_library(&fixture.0, names.clone())?;
        let shared = parsed.block_library(&fixture.0, names)?;
        for name in ["horizon:nested/child", GENERATED_MODEL, MISSING_MODEL] {
            assert_eq!(
                format!("{:?}", old.resolve(name)?),
                format!("{:?}", shared.resolve(name)?)
            );
        }
        assert!(shared.resolve("horizon:a").is_err());
        for name in ["horizon:broken", "horizon:absent", "horizon:dangling"] {
            let load = |models: &ParsedModels| {
                models
                    .block_library(&fixture.0, vec![name.into()])
                    .err()
                    .unwrap()
                    .to_string()
            };
            assert_eq!(load(&direct), load(&parsed));
            // Mods' content: vanilla's missing model instead of an error.
            let lenient = parsed.block_library_or_missing(&fixture.0, vec![name.into()])?;
            assert_eq!(
                format!("{:?}", lenient.resolve(name)?.elements()),
                format!("{:?}", lenient.resolve(MISSING_MODEL)?.elements()),
                "{name}"
            );
        }
        let items = parsed.into_item_library()?;
        let block = shared.resolve("horizon:nested/child")?;
        let item = items.resolve("horizon:nested/child")?;
        assert!(std::ptr::eq(
            block.elements().as_ptr(),
            item.elements().as_ptr()
        ));
        assert_eq!(
            item.material("side")?.unwrap().sprite,
            "minecraft:block/glass"
        );
        assert_eq!(shared.resolve(GENERATED_MODEL)?.elements().len(), 1);
        assert!(items.resolve(GENERATED_MODEL)?.elements().is_empty());
        assert!(items.resolve(GENERATED_MODEL)?.uses_item_generator());
        for name in [
            "horizon:broken",
            "horizon:absent",
            "horizon:dangling",
            MISSING_MODEL,
        ] {
            assert_eq!(items.resolve(name)?.elements().len(), 1);
        }
        assert!(items.resolve("horizon:a").is_err());
        fixture.write("horizon:parent", r#"{"elements":[]}"#);
        let reloaded = ParsedModels::load(&fixture.0).into_item_library()?;
        assert!(
            reloaded
                .resolve("horizon:nested/child")?
                .elements()
                .is_empty()
        );
        assert_eq!(items.resolve("horizon:nested/child")?.elements().len(), 1);
        // An unrelated bad item identifier must not make block preparation fail
        // (which would trigger resource-pack rollback at a different stage).
        fixture.write("horizon:INVALID", "{}");
        let parsed = ParsedModels::load(&fixture.0);
        assert!(
            parsed
                .block_library(&fixture.0, vec!["horizon:nested/child".into()])
                .is_ok()
        );
        assert!(parsed.into_item_library().is_err());
        Ok(())
    }
}
