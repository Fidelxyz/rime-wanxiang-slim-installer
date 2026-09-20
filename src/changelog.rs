use anyhow::{Context, Result};
use regex::Regex;
use semver::Version;
use std::collections::HashSet;

pub fn breaking_changes(
    changelog: &str,
    installed: &Version,
    latest: &Version,
) -> Result<Vec<(Version, String)>> {
    let version_regex = Regex::new(r"^\[?v?([^\]\s]+)").unwrap();

    let mut changes = Vec::new();
    let mut stable_versions = HashSet::new();
    let mut version = None;
    let mut breaking = false;
    let mut body = String::new();

    for line in changelog.lines().chain(std::iter::once("## ")) {
        if let Some(heading) = line.strip_prefix("## ") {
            if let Some(version) = version.take()
                && !body.trim().is_empty()
            {
                changes.push((version, body.trim().to_owned()));
            }

            body.clear();
            breaking = false;

            if heading.is_empty() {
                continue;
            }

            let captures = version_regex
                .captures(heading)
                .with_context(|| format!("无法解析 CHANGELOG 标题：{heading}"))?;
            let (_, [text]) = captures.extract();
            let parsed =
                Version::parse(text).with_context(|| format!("无法解析 CHANGELOG 版本：{text}"))?;
            if parsed > *installed && parsed <= *latest {
                if parsed.pre.is_empty() {
                    stable_versions.insert((parsed.major, parsed.minor, parsed.patch));
                }
                version = Some(parsed);
            }
        } else if let Some(heading) = line.strip_prefix("### ") {
            breaking = heading.trim().ends_with("BREAKING CHANGES");
        } else if version.is_some() && breaking {
            body.push_str(line);
            body.push('\n');
        }
    }
    changes.retain(|(version, _)| {
        version.pre.is_empty()
            || !stable_versions.contains(&(version.major, version.minor, version.patch))
    });
    Ok(changes)
}
