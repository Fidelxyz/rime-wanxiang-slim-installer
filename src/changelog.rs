use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::regex;
use semver::Version;

pub const BREAKING_CHANGES_HEADING: &str = "⚠ BREAKING CHANGES";

const SECTION_ORDER: &[&str] = &[
    BREAKING_CHANGES_HEADING,
    "Features",
    "Bug Fixes",
    "Performance Improvements",
    "Reverts",
    "Documentation",
    "Styles",
    "Miscellaneous Chores",
    "Code Refactoring",
    "Tests",
    "Build System",
    "Continuous Integration",
];

pub fn collect_changes(
    changelog: &str,
    installed: &Version,
    latest: &Version,
) -> Result<Vec<(String, String)>> {
    let mut sections: IndexMap<String, Vec<String>> = SECTION_ORDER
        .iter()
        .map(|&heading| (heading.to_owned(), Vec::new()))
        .collect();
    let mut in_range = false;
    let mut section = None;

    for line in changelog.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            let version = parse_version_heading(heading)?;
            in_range = version > *installed && version <= *latest;
            section = None;
        } else if let Some(heading) = line.strip_prefix("### ") {
            section = in_range.then(|| heading.trim().to_owned());
        } else if let Some(heading) = &section
            && !line.trim().is_empty()
        {
            let entry = regex!(r"(\s*\(\[[^\]]+\]\([^)]+\)\))+$").replace(line, "");
            sections
                .entry(heading.clone())
                .or_default()
                .push(entry.into_owned());
        }
    }

    Ok(sections
        .into_iter()
        .filter(|(_, entries)| !entries.is_empty())
        .map(|(heading, entries)| (heading, entries.join("\n")))
        .collect())
}

fn parse_version_heading(heading: &str) -> Result<Version> {
    let text = heading.trim_start_matches('[');
    let text = text
        .split_once([']', ' '])
        .map_or(text, |(version, _)| version);
    Version::parse(text).with_context(|| format!("无法解析 CHANGELOG 版本：{heading}"))
}
