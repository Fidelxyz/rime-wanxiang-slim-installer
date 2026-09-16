use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use octocrab::{self, models::repos::Asset};
use reqwest::Client;
use std::{
    io::{self, Seek, Write},
    time::Duration,
};
use tempfile::NamedTempFile;

use crate::digest;

pub struct Downloader {
    client: Client,
}

impl Downloader {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent(concat!(
                "rime-wanxiang-slim-installer/",
                env!("CARGO_PKG_VERSION")
            ))
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_hours(1))
            .build()?;
        Ok(Self { client })
    }

    pub async fn download(&self, asset: &Asset) -> Result<NamedTempFile> {
        let mut tempfile = NamedTempFile::new()?;

        println!("下载 {}", asset.name);

        let mut response = self
            .client
            .get(asset.browser_download_url.clone())
            .send()
            .await?
            .error_for_status()?;

        let total = response.content_length();
        let progress = ProgressBar::new(total.unwrap_or(0)).with_style(
            ProgressStyle::with_template("[{percent:>3}%] {wide_bar} {bytes} / {total_bytes}")
                .unwrap(),
        );
        while let Some(chunk) = response.chunk().await? {
            tempfile.write_all(&chunk)?;
            progress.inc(chunk.len() as u64);
        }
        progress.finish();

        tempfile.flush()?;
        tempfile.seek(io::SeekFrom::Start(0))?;
        digest::verify(tempfile.as_file(), asset)?;

        progress.finish_and_clear();
        Ok(tempfile)
    }
}
