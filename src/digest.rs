use anyhow::{Context, Result, bail};
use crossterm::style::Stylize;
use octocrab::{self, models::repos::Asset};

pub fn verify(file_digest: &[u8], asset: &Asset) -> Result<()> {
    let file_digest = base16ct::lower::encode_string(file_digest);
    let asset_digest = asset_digest(asset)?;
    if file_digest != asset_digest {
        eprintln!("{}", format!("本地 SHA-256：{file_digest}").dark_red());
        eprintln!("{}", format!("远端 SHA-256：{asset_digest}").dark_red());
        bail!("{} SHA-256 不匹配", asset.name);
    }
    Ok(())
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
