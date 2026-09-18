use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::{fs, path::Path};
use strum::IntoEnumIterator;

use crate::network::Network;
use crate::options::{AuxMode, Pinyin, Schema};
use crate::workflow::{ApplyFuture, Module};
use crate::yaml::Document;

pub struct Apply {
    pub schema: Schema,
    pub config: Config,
}

impl Module for Apply {
    fn info(&self) {
        info_apply(self.config);
    }

    fn warn(&self, root: &Path) -> Result<()> {
        warn_apply(root, self.schema);
        Ok(())
    }

    fn apply<'a>(self: Box<Self>, _downloader: &'a Network, root: &'a Path) -> ApplyFuture<'a> {
        Box::pin(async move {
            apply(root, self.schema, self.config).context("应用方案配置失败")
        })
    }
}

#[derive(Clone, Copy, PartialEq)]
pub struct Config {
    pub pinyin: Option<Pinyin>,
    pub aux_mode: Option<AuxMode>,
}

fn info_apply(config: Config) {
    println!("{} 将应用方案配置：", "==>".bright_green());
    if let Some(pinyin) = config.pinyin {
        println!("  拼音方案：{}", pinyin.to_string().bright_cyan());
    }
    if let Some(aux_mode) = config.aux_mode {
        println!("  辅助码方案：{}", aux_mode.to_string().bright_cyan());
    }
}

fn warn_apply(root: &Path, schema: Schema) {
    for schema_id in [schema.schema_id(), "wanxiang_reverse"] {
        let file = format!("{schema_id}.custom.yaml");
        let target = root.join(&file);
        println!(
            "{}",
            if target.exists() {
                format!("将修改配置文件：{}", target.display())
            } else {
                format!("将创建配置文件：{}", target.display())
            }
            .bright_yellow()
        );
    }
    println!(
        "{}",
        format!(
            "将修改配置文件：{}",
            root.join(format!("{}.custom.yaml", schema.schema_id()))
                .display()
        )
        .bright_yellow()
    );
}

fn apply(root: &Path, schema: Schema, config: Config) -> Result<()> {
    let mut pending_writes = vec![];

    for (schema_id, group) in [
        (schema.schema_id(), schema.code()),
        ("wanxiang_reverse", "reverse"),
    ] {
        let file = format!("{schema_id}.custom.yaml");
        let target = root.join(&file);
        let source = if target.exists() {
            target.clone()
        } else {
            root.join("custom").join(&file)
        };

        let text = fs::read_to_string(&source)
            .with_context(|| format!("无法读取文件 {}", source.display()))?;
        pending_writes.push((target, rewrite(&text, group, config)?));
    }

    for (target, text) in pending_writes {
        fs::write(target, text)?;
    }

    println!("{}", "已应用方案配置。".bright_green());
    Ok(())
}

fn rewrite(source: &str, group: &str, config: Config) -> Result<String> {
    let document = Document::read(source.as_bytes())?;
    // YAML spans use character indices; String replacements use byte offsets.
    let byte_offsets: Vec<_> = source
        .char_indices()
        .map(|(offset, _)| offset)
        .chain(std::iter::once(source.len()))
        .collect();
    let reference_prefix = format!("wanxiang_algebra:/{group}/");

    let mut found_pinyin = false;
    let mut found_aux_mode = false;
    let mut edits = vec![];

    for field in [
        ["patch", "speller/algebra", "__patch"].as_slice(),
        ["patch", "speller/algebra", "__include"].as_slice(),
        ["patch", "speller/algebra/__patch"].as_slice(),
        ["patch", "speller/algebra/__include"].as_slice(),
    ]
    .into_iter()
    .flat_map(|path| document.values(path))
    {
        let Some(old) = field
            .data
            .as_str()
            .and_then(|value| value.strip_prefix(&reference_prefix))
        else {
            continue;
        };

        let new = if Pinyin::iter().any(|value| value.to_string() == old) {
            found_pinyin = true;
            config.pinyin.map(|value| value.to_string())
        } else if AuxMode::iter().any(|value| value.to_string() == old) {
            found_aux_mode = true;
            config.aux_mode.map(|value| value.to_string())
        } else {
            None
        };

        if let Some(new) = new.filter(|value| value != old) {
            let range =
                byte_offsets[field.span.start.index()]..byte_offsets[field.span.end.index()];
            edits.push((range, format!("{reference_prefix}{new}")));
        }
    }

    if !found_pinyin {
        bail!("未在自定义文件中找到拼音方案引用，请检查自定义文件是否完整")
    }
    if config.aux_mode.is_some() && !found_aux_mode {
        bail!("未在自定义文件中找到辅助码方案引用，请检查自定义文件是否完整")
    }

    // Apply edits from the end so replacements don't shift earlier offsets.
    edits.sort_unstable_by_key(|(range, _)| std::cmp::Reverse(range.start));
    let mut rewritten = source.to_owned();
    for (range, replacement) in edits {
        rewritten.replace_range(range, &replacement);
    }
    Ok(rewritten)
}
