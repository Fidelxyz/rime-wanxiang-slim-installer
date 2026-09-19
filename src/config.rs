use anyhow::{Context, Result};
use std::{fs, io::ErrorKind, path::Path};

use crate::INSTALLER_DIR;
use crate::yaml::Document;

#[derive(Default)]
pub struct Config {
    pub prerelease: bool,
}

const CONFIG_FILE: &str = "config.yaml";

impl Config {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(INSTALLER_DIR).join(CONFIG_FILE);
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => {
                return Err(error).with_context(|| format!("无法打开配置文件 {}", path.display()));
            }
        };
        let document =
            Document::read(file).with_context(|| format!("无法解析配置文件 {}", path.display()))?;
        let prerelease = document
            .required(&["prerelease"])?
            .data
            .as_bool()
            .context("prerelease 字段类型必须为 bool")?;
        Ok(Self { prerelease })
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let directory = root.join(INSTALLER_DIR);
        fs::create_dir_all(&directory)
            .with_context(|| format!("无法创建目录 {}", directory.display()))?;
        let path = directory.join(CONFIG_FILE);
        fs::write(&path, format!("prerelease: {}\n", self.prerelease))
            .with_context(|| format!("无法写入配置文件 {}", path.display()))
    }
}
