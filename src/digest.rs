use anyhow::{Context, Result, bail};
use colored::Colorize;
use octocrab::{self, models::repos::Asset};
use sha2::{Digest, Sha256};
use std::{fs::File, io};

pub fn verify(file: &File, asset: &Asset) -> Result<()> {
    let file_digest = file_digest(file)?;
    let asset_digest = asset_digest(asset)?;
    if file_digest != asset_digest {
        eprintln!("{}", "本地 SHA-256：{file_digest}".red());
        eprintln!("{}", "远端 SHA-256：{asset_digest}".red());
        bail!("{} SHA-256 不匹配", asset.name);
    }
    Ok(())
}

fn file_digest(file: &File) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut reader = io::BufReader::new(file);
    io::copy(&mut reader, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn asset_digest(asset: &Asset) -> Result<String> {
    Ok(asset
        .digest
        .as_ref()
        .context("远端未提供 SHA-256 摘要")?
        .strip_prefix("sha256:")
        .context("远端 SHA-256 摘要格式不正确")?
        .to_string())
}
