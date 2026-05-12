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

pub fn scan_m3u_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut result = Vec::new();
    collect_m3u_files_recursive(dir, &mut result)?;

    result.sort();
    Ok(result)
}

fn collect_m3u_files_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir)
        .map_err(|e| format!("failed to read directory {}: {e}", dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|e| format!("failed to read entry in {}: {e}", dir.display()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_m3u_files_recursive(&path, out)?;
            continue;
        }

        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("m3u") || ext.eq_ignore_ascii_case("m3u8"))
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
}
