use anyhow::Context;
use regex::Regex;
use std::{
    fs,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;

use crate::grammar::GRAMMAR_NAME;
use crate::options::{AuxCode, AuxMode, Pinyin, Scheme};
use crate::print_err;
use crate::yaml::Document;

#[derive(Clone)]
pub struct InstalledSchema {
    pub scheme: Scheme,
    pub version: String,
    pub pinyin: Option<Pinyin>,
    pub aux_mode: Option<AuxMode>,
}

impl InstalledSchema {
    pub fn detect(root: &Path) -> Vec<Self> {
        let aux_code_assignment_regex =
            Regex::new(r#"(?m)^\s*M\.AUX_CODE\s*=\s*["']([^"']+)["']"#).unwrap();
        let algebra_reference_regex = Regex::new(r"^wanxiang_algebra:/(?:base|pro)/(.+)$").unwrap();

        let mut installed = vec![];
        for scheme in Scheme::iter() {
            let schema_id = scheme.schema_id();
            let path = root.join(format!("{schema_id}.schema.yaml"));
            if !path.exists() {
                continue;
            }

            let schema_document = match Document::open(&path)
                .with_context(|| format!("无法读取方案文件 {}", path.display()))
            {
                Ok(document) => document,
                Err(e) => {
                    print_err(e);
                    continue;
                }
            };
            let version = match schema_document
                .required(&["schema", "version"])
                .and_then(|field| {
                    field
                        .data
                        .as_str()
                        .context("方案版本字段不是字符串")
                        .map(str::to_owned)
                })
                .with_context(|| format!("无法读取方案版本 {}", path.display()))
            {
                Ok(version) => version,
                Err(e) => {
                    print_err(e);
                    continue;
                }
            };

            // Read algebra patches from custom.yaml if it exists
            let custom_path = root.join(format!("{schema_id}.custom.yaml"));
            let mut algebra_patches = if custom_path.exists() {
                let custom_document = match Document::open(&custom_path)
                    .with_context(|| format!("无法读取自定义文件 {}", custom_path.display()))
                {
                    Ok(document) => document,
                    Err(e) => {
                        print_err(e);
                        continue;
                    }
                };
                custom_document
                    .values(&["patch", "speller/algebra", "__patch"])
                    .into_iter()
                    .filter_map(|field| field.data.as_str())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };

            // Read algebra patches from schema.yaml if not found in custom.yaml
            if algebra_patches.is_empty() {
                algebra_patches = schema_document
                    .values(&["speller", "algebra", "__patch"])
                    .into_iter()
                    .filter_map(|field| field.data.as_str())
                    .map(str::to_owned)
                    .collect();
            }

            // Extract reference names from algebra patches
            let names: Vec<_> = algebra_patches
                .iter()
                .filter_map(|field| {
                    algebra_reference_regex
                        .captures(field)
                        .map(|capture| capture.get(1).unwrap().as_str())
                })
                .collect();

            // Determine pinyin and aux mode based on the extracted names
            let pinyin = Pinyin::iter().find(|value| names.contains(&value.to_string().as_str()));
            let aux_mode = if names.contains(&"间接辅助") {
                Some(AuxMode::Indirect)
            } else if names.contains(&"直接辅助") {
                Some(AuxMode::Direct)
            } else {
                None
            };

            // Read aux code from meta.lua if it exists
            let aux = match scheme {
                Scheme::Base => None,
                Scheme::Pro(_) => {
                    fs::read_to_string(root.join("lua/meta.lua"))
                        .ok()
                        .and_then(|meta| {
                            aux_code_assignment_regex
                                .captures(&meta)
                                .and_then(|c| AuxCode::iter().find(|value| value.code() == &c[1]))
                        })
                }
            };

            let scheme = match scheme {
                Scheme::Base => Scheme::Base,
                Scheme::Pro(_) => Scheme::Pro(aux),
            };

            installed.push(Self {
                scheme,
                version,
                pinyin,
                aux_mode,
            });
        }
        installed
    }
}

pub struct InstalledGrammar {
    pub path: PathBuf,
}

impl InstalledGrammar {
    pub fn detect(root: &Path) -> Option<Self> {
        let path = root.join(GRAMMAR_NAME);
        if !path.exists() {
            return None;
        }
        Some(Self { path })
    }
}
