use anyhow::{Context, Result};
use saphyr::{LoadableYamlNode, MarkedYamlOwned, YamlDataOwned};
use std::{
    fs::File,
    io::{self, Read},
    iter,
    path::Path,
    slice::Iter,
};

pub struct Document {
    root: MarkedYamlOwned,
}

fn iter_fields<'a, 'p>(
    node: &'a MarkedYamlOwned,
    mut path: Iter<'p, &str>,
) -> Box<dyn Iterator<Item = &'a MarkedYamlOwned> + 'p>
where
    'a: 'p,
{
    if let Some(mapping) = node.data.as_mapping() {
        Box::new(
            path.next()
                .and_then(|key| mapping.get(&MarkedYamlOwned::value_from_str(key)))
                .into_iter()
                .flat_map(move |value| iter_fields(value, path.clone())),
        )
    } else if let Some(sequence) = node.data.as_sequence() {
        Box::new(
            sequence
                .iter()
                .flat_map(move |value| iter_fields(value, path.clone())),
        )
    } else if path.len() == 0 && matches!(&node.data, YamlDataOwned::Value(_)) {
        Box::new(iter::once(node))
    } else {
        Box::new(iter::empty())
    }
}

impl Document {
    pub fn open(path: &Path) -> Result<Self> {
        Self::read(File::open(path)?)
    }

    pub fn read(reader: impl Read) -> Result<Self> {
        let source = io::read_to_string(reader)?;
        let root = MarkedYamlOwned::load_from_str(&source)?
            .into_iter()
            .next()
            .context("YAML 文档为空")?;
        Ok(Self { root })
    }

    pub fn values(&self, path: &[&str]) -> Vec<&MarkedYamlOwned> {
        iter_fields(&self.root, path.iter()).collect()
    }

    pub fn required(&self, path: &[&str]) -> Result<&MarkedYamlOwned> {
        iter_fields(&self.root, path.iter())
            .next()
            .with_context(|| format!("YAML 文档缺少字段 {}", path.join(".")))
    }
}
