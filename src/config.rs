use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::{fs, path::Path};
use strum::IntoEnumIterator;

use crate::options::{AuxMode, Pinyin, Scheme};
use crate::yaml::Document;

fn rewrite(source: &str, group: &str, pinyin: Option<&str>, mode: Option<&str>) -> Result<String> {
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

        let new = if AuxMode::iter().any(|value| value.to_string() == old) {
            found_aux_mode = true;
            mode
        } else if Pinyin::iter().any(|value| value.to_string() == old) {
            found_pinyin = true;
            pinyin
        } else {
            None
        };

        if let Some(new) = new.filter(|&value| value != old) {
            let range =
                byte_offsets[field.span.start.index()]..byte_offsets[field.span.end.index()];
            edits.push((range, format!("{reference_prefix}{new}")));
        }
    }

    if pinyin.is_some() && !found_pinyin {
        bail!("未在自定义文件中找到拼音方案引用，请检查自定义文件是否完整")
    }
    if mode.is_some() && !found_aux_mode {
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

pub fn apply(
    root: &Path,
    scheme: Scheme,
    pinyin: Option<Pinyin>,
    mode: Option<AuxMode>,
) -> Result<()> {
    let pinyin = pinyin.map(|value| value.to_string());
    let mode = mode.map(|value| value.to_string());

    let mut pending_writes = vec![];

    for (schema_id, group, pinyin, mode) in [
        (
            scheme.schema_id(),
            scheme.code(),
            pinyin.as_deref(),
            mode.as_deref(),
        ),
        ("wanxiang_reverse", "reverse", pinyin.as_deref(), None),
    ] {
        if pinyin.is_none() && mode.is_none() {
            continue;
        }

        let file = format!("{schema_id}.custom.yaml");
        let target = root.join(&file);
        let source = if target.exists() {
            target.clone()
        } else {
            root.join("custom").join(&file)
        };

        let text = fs::read_to_string(&source)
            .with_context(|| format!("无法读取文件 {}", source.display()))?;
        pending_writes.push((target, rewrite(&text, group, pinyin, mode)?));
    }

    for (target, text) in pending_writes {
        fs::write(target, text)?;
    }

    println!("{}", "已应用方案配置。".bright_green());
    Ok(())
}
