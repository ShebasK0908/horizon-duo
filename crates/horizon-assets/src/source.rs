use std::{borrow::Cow, collections::BTreeMap, io, path::Path};

pub trait ResourceSource: Sync {
    fn read(&self, path: &str) -> io::Result<Cow<'_, [u8]>>;
    fn files(&self, prefix: &str) -> io::Result<Vec<String>>;
}

pub struct NativeResources<'a>(pub &'a Path);
impl ResourceSource for NativeResources<'_> {
    fn read(&self, path: &str) -> io::Result<Cow<'_, [u8]>> {
        validate_path(path)?;
        std::fs::read(self.0.join(path)).map(Cow::Owned)
    }
    fn files(&self, prefix: &str) -> io::Result<Vec<String>> {
        validate_path(prefix)?;
        let mut pending = vec![self.0.join(prefix)];
        let mut files = Vec::new();
        while let Some(dir) = pending.pop() {
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    let relative = path.strip_prefix(self.0).map_err(io::Error::other)?;
                    files.push(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
        files.sort();
        Ok(files)
    }
}

pub struct ResourceBundle {
    files: BTreeMap<String, Vec<u8>>,
}
impl ResourceBundle {
    pub fn new(files: BTreeMap<String, Vec<u8>>) -> io::Result<Self> {
        for path in files.keys() {
            validate_path(path)?;
        }
        Ok(Self { files })
    }
}
impl ResourceSource for ResourceBundle {
    fn read(&self, path: &str) -> io::Result<Cow<'_, [u8]>> {
        validate_path(path)?;
        self.files
            .get(path)
            .map(|bytes| Cow::Borrowed(bytes.as_slice()))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, path.to_owned()))
    }
    fn files(&self, prefix: &str) -> io::Result<Vec<String>> {
        validate_path(prefix)?;
        let prefix = format!("{}/", prefix.trim_end_matches('/'));
        Ok(self
            .files
            .range(prefix.clone()..)
            .take_while(|(path, _)| path.starts_with(&prefix))
            .map(|(path, _)| path.clone())
            .collect())
    }
}
fn validate_path(path: &str) -> io::Result<()> {
    if path.is_empty()
        || path.contains('\\')
        || path.contains(':')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid resource path",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundle_reads_exact_paths_and_lists_descendants_in_order() {
        let bundle = ResourceBundle::new(BTreeMap::from([
            ("assets/test/models/b.json".into(), vec![2]),
            ("assets/test/models/a.json".into(), vec![1]),
            ("assets/testing/models/a.json".into(), vec![3]),
        ]))
        .unwrap();
        assert_eq!(
            bundle.read("assets/test/models/a.json").unwrap().as_ref(),
            &[1]
        );
        assert_eq!(bundle.files("assets/test").unwrap().len(), 2);
        assert_eq!(
            bundle.read("missing").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
    }
    #[test]
    fn bundle_rejects_escaping_and_ambiguous_paths() {
        for path in [
            "/absolute",
            "../escape",
            "a/../b",
            "a//b",
            "a/./b",
            "C:/data",
            "a\\b",
        ] {
            assert!(ResourceBundle::new(BTreeMap::from([(path.into(), vec![])])).is_err());
        }
    }
}
