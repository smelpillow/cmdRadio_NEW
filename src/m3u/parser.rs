use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Station {
    pub name: String,
    pub url: String,
}

pub fn parse_m3u_file(path: &Path) -> Result<Vec<Station>, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let mut stations = Vec::new();
    let mut pending_name: Option<String> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("#EXTINF:") {
            let name = rest
                .split_once(',')
                .map(|(_, n)| n.trim().to_owned())
                .filter(|n| !n.is_empty());
            pending_name = name;
            continue;
        }

        if trimmed.starts_with('#') {
            continue;
        }

        if is_http_url(trimmed) {
            let name = pending_name
                .take()
                .unwrap_or_else(|| format!("Station {}", stations.len() + 1));
            stations.push(Station {
                name,
                url: trimmed.to_owned(),
            });
        }
    }

    Ok(stations)
}

pub fn parse_pls_file(path: &Path) -> Result<Vec<Station>, String> {
    let raw =
        fs::read_to_string(path).map_err(|e| format!("failed to read {}: {e}", path.display()))?;

    let mut file_map: std::collections::BTreeMap<usize, String> = BTreeMap::new();
    let mut title_map: std::collections::BTreeMap<usize, String> = BTreeMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('[') {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();
        if value.is_empty() {
            continue;
        }

        if let Some(index_text) = key.strip_prefix("File") {
            if let Ok(index) = index_text.parse::<usize>()
                && (value.starts_with("http://") || value.starts_with("https://"))
            {
                file_map.insert(index, value.to_string());
            }
            continue;
        }

        if let Some(index_text) = key.strip_prefix("Title") {
            if let Ok(index) = index_text.parse::<usize>() {
                title_map.insert(index, value.to_string());
            }
        }
    }

    let mut stations = Vec::new();
    for (index, url) in file_map {
        let name = title_map
            .get(&index)
            .cloned()
            .filter(|name: &String| !name.trim().is_empty())
            .unwrap_or_else(|| format!("Station {}", stations.len() + 1));
        stations.push(Station { name, url });
    }

    Ok(stations)
}

pub fn scan_m3u_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    let mut visited_dirs = HashSet::new();
    collect_m3u_files_recursive(dir, &mut result, &mut visited_dirs)?;

    result.sort();
    Ok(result)
}

fn collect_m3u_files_recursive(
    dir: &Path,
    out: &mut Vec<PathBuf>,
    visited_dirs: &mut HashSet<PathBuf>,
) -> Result<(), String> {
    let canonical_dir = fs::canonicalize(dir)
        .map_err(|e| format!("failed to resolve directory {}: {e}", dir.display()))?;
    if !visited_dirs.insert(canonical_dir) {
        return Ok(());
    }

    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read entry in {}: {e}", dir.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_m3u_files_recursive(&path, out, visited_dirs)?;
            continue;
        }

        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                ext.eq_ignore_ascii_case("m3u")
                    || ext.eq_ignore_ascii_case("m3u8")
                    || ext.eq_ignore_ascii_case("pls")
            })
            .unwrap_or(false)
        {
            out.push(path);
        }
    }

    Ok(())
}

fn is_http_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_m3u() {
        let tmp = std::env::temp_dir().join("cmdradio_parser_test.m3u");
        let content = "#EXTM3U\n#EXTINF:-1,Sample Radio\nhttps://stream.example.org/live.mp3\n";
        fs::write(&tmp, content).expect("test file write");

        let parsed = parse_m3u_file(&tmp).expect("parse should work");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Sample Radio");
        assert_eq!(parsed[0].url, "https://stream.example.org/live.mp3");

        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn scan_m3u_files_includes_nested_directories() {
        let root = std::env::temp_dir().join("cmdradio_scan_recursive_test");
        let nested = root.join("nested").join("deep");

        fs::create_dir_all(&nested).expect("nested directories created");
        fs::write(root.join("top.m3u"), "#EXTM3U\n").expect("top file created");
        fs::write(nested.join("deep.m3u8"), "#EXTM3U\n").expect("deep file created");
        fs::write(root.join("ignore.txt"), "not playlist").expect("non playlist created");

        let scanned = scan_m3u_files(&root).expect("scan should work");

        assert_eq!(scanned.len(), 2);
        assert!(scanned.iter().any(|p| p.ends_with("top.m3u")));
        assert!(scanned.iter().any(|p| p.ends_with("deep.m3u8")));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn parse_ignores_non_http_and_uses_fallback_station_names() {
        let tmp = std::env::temp_dir().join("cmdradio_parser_non_http_test.m3u");
        let content = "#EXTM3U\n#EXTINF:-1,Named Entry\nftp://example.org/not_supported\nhttps://stream.example.org/aac\nhttp://stream.example.org/mp3\n";
        fs::write(&tmp, content).expect("test file write");

        let parsed = parse_m3u_file(&tmp).expect("parse should work");
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "Named Entry");
        assert_eq!(parsed[0].url, "https://stream.example.org/aac");
        assert_eq!(parsed[1].name, "Station 2");
        assert_eq!(parsed[1].url, "http://stream.example.org/mp3");

        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn parse_extinf_without_name_falls_back_to_default() {
        let tmp = std::env::temp_dir().join("cmdradio_parser_extinf_fallback_test.m3u");
        let content = "#EXTM3U\n#EXTINF:-1,\nhttps://stream.example.org/live\n";
        fs::write(&tmp, content).expect("test file write");

        let parsed = parse_m3u_file(&tmp).expect("parse should work");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Station 1");
        assert_eq!(parsed[0].url, "https://stream.example.org/live");

        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn parse_pls_file_supports_local_station_lists() {
        let tmp = std::env::temp_dir().join("cmdradio_parser_pls_test.pls");
        let content = "[playlist]\nFile1=https://stream.example.org/live\nTitle1=Rock FM\nLength1=-1\nNumberOfEntries=1\nVersion=2\n";
        fs::write(&tmp, content).expect("test file write");

        let parsed = parse_pls_file(&tmp).expect("PLS parse should work");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "Rock FM");
        assert_eq!(parsed[0].url, "https://stream.example.org/live");

        let _ = fs::remove_file(tmp);
    }

    #[test]
    fn scan_m3u_files_includes_pls_playlists() {
        let root = std::env::temp_dir().join("cmdradio_scan_pls_test");
        fs::create_dir_all(&root).expect("root dir created");
        fs::write(root.join("stations.pls"), "[playlist]\nFile1=https://stream.example.org/live\n").expect("pls file created");
        fs::write(root.join("another.m3u"), "#EXTM3U\nhttps://stream2.example.org/live\n").expect("m3u file created");

        let scanned = scan_m3u_files(&root).expect("scan should work");

        assert_eq!(scanned.len(), 2);
        assert!(scanned.iter().any(|p| p.ends_with("stations.pls")));
        assert!(scanned.iter().any(|p| p.ends_with("another.m3u")));

        let _ = fs::remove_dir_all(root);
    }
}
