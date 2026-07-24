// src/scaffold/archive.rs — Source directory → tar.gz archive.
//
// The archive is uploaded to the API for agent and template builds.
// Exclusion patterns exactly match the Python SDK `_ARCHIVE_EXCLUDE_PATTERNS`
// from `gravixlayer-python/src/gravixlayer/resources/agents.py`.

use std::io::{self};
use std::path::Path;

use flate2::{write::GzEncoder, Compression};
use walkdir::{DirEntry, WalkDir};

// Patterns to exclude when building the archive.  These exactly mirror the
// Python SDK's `_ARCHIVE_EXCLUDE_PATTERNS` frozenset.
pub const BUILTIN_EXCLUDES: &[&str] = &[
    "__pycache__",
    ".git",
    ".venv",
    "venv",
    "env",
    ".env",
    "node_modules",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    ".tox",
    "dist",
    "build",
    ".DS_Store",
];

// Suffix-based exclusions (Python SDK patterns starting with `*`).
const BUILTIN_SUFFIX_EXCLUDES: &[&str] = &[".egg-info"];

/// Create an in-memory tar.gz archive from a source directory.
///
/// `extra_excludes` allows callers (e.g., `gravixlayer agent build`) to pass
/// additional patterns from the project's `gravixlayer.json` `exclude` field.
///
/// # Errors
/// Returns an `io::Error` if any file cannot be read or the archive cannot be
/// written to the in-memory buffer.
pub fn create_source_archive(source: &Path, extra_excludes: &[String]) -> io::Result<Vec<u8>> {
    if !source.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("source is not a directory: {}", source.display()),
        ));
    }

    let buf = Vec::new();
    let enc = GzEncoder::new(buf, Compression::default());
    let mut tar = tar::Builder::new(enc);

    for entry in WalkDir::new(source).sort_by_file_name().into_iter() {
        let entry: DirEntry = entry?;
        let abs_path = entry.path();
        let rel_path = abs_path
            .strip_prefix(source)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        if should_exclude(rel_path, extra_excludes) {
            continue;
        }

        if abs_path.is_file() {
            tar.append_path_with_name(abs_path, rel_path)?;
        } else if abs_path.is_dir() && rel_path != Path::new("") {
            tar.append_dir(rel_path, abs_path)?;
        }
    }

    let gz = tar.into_inner()?.finish()?;
    Ok(gz)
}

/// Return `true` if this relative path should be excluded from the archive.
fn should_exclude(rel: &Path, extra_excludes: &[String]) -> bool {
    for component in rel.components() {
        let part = component.as_os_str().to_string_lossy();

        // Check exact component matches against builtin list.
        if BUILTIN_EXCLUDES.contains(&part.as_ref()) {
            return true;
        }

        // Check suffix-based builtin patterns (e.g. *.egg-info).
        if BUILTIN_SUFFIX_EXCLUDES
            .iter()
            .any(|suf| part.ends_with(suf))
        {
            return true;
        }

        // Check extra patterns supplied by the project file.
        for pat in extra_excludes {
            let pat = pat.trim_start_matches('*');
            if part.ends_with(pat) || part.as_ref() == pat.trim_start_matches('/') {
                return true;
            }
        }
    }
    false
}

/// Return the uncompressed byte count for a proposed archive without actually
/// creating it (dry-run helper used by `gravixlayer package --dry-run`).
pub fn estimate_archive_size(source: &Path, extra_excludes: &[String]) -> io::Result<u64> {
    let mut total = 0u64;
    for entry in WalkDir::new(source).into_iter() {
        let entry: DirEntry = entry?;
        let rel = entry
            .path()
            .strip_prefix(source)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        if should_exclude(rel, extra_excludes) {
            continue;
        }
        if entry.path().is_file() {
            total += entry.metadata()?.len();
        }
    }
    Ok(total)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Cursor;
    use tempfile::TempDir;

    fn setup_project(dir: &Path) {
        // Normal source files
        fs::write(dir.join("main.py"), b"print('hello')").unwrap();
        fs::write(dir.join("requirements.txt"), b"requests\n").unwrap();

        // Files that must be excluded
        let pycache = dir.join("__pycache__");
        fs::create_dir_all(&pycache).unwrap();
        fs::write(pycache.join("main.cpython-312.pyc"), b"\x00").unwrap();

        let venv = dir.join(".venv");
        fs::create_dir_all(&venv).unwrap();
        fs::write(venv.join("pyvenv.cfg"), b"home = /usr\n").unwrap();

        let egg = dir.join("mypackage.egg-info");
        fs::create_dir_all(&egg).unwrap();
        fs::write(egg.join("PKG-INFO"), b"Name: mypackage\n").unwrap();

        let git = dir.join(".git");
        fs::create_dir_all(&git).unwrap();
        fs::write(git.join("HEAD"), b"ref: refs/heads/main\n").unwrap();
    }

    #[test]
    fn archive_excludes_standard_patterns() {
        let dir = TempDir::new().unwrap();
        setup_project(dir.path());

        let archive_bytes = create_source_archive(dir.path(), &[]).unwrap();
        assert!(!archive_bytes.is_empty(), "archive should not be empty");

        // Decode the archive and collect entry names
        let cursor = Cursor::new(&archive_bytes);
        let gz = flate2::read::GzDecoder::new(cursor);
        let mut tar = tar::Archive::new(gz);
        let entries: Vec<String> = tar
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().into_owned())
            .collect();

        // Must include
        assert!(
            entries.iter().any(|p| p.ends_with("main.py")),
            "main.py missing"
        );
        assert!(
            entries.iter().any(|p| p.ends_with("requirements.txt")),
            "requirements.txt missing"
        );

        // Must exclude
        assert!(
            !entries.iter().any(|p| p.contains("__pycache__")),
            "__pycache__ should be excluded"
        );
        assert!(
            !entries.iter().any(|p| p.contains(".venv")),
            ".venv should be excluded"
        );
        assert!(
            !entries.iter().any(|p| p.contains(".egg-info")),
            "*.egg-info should be excluded"
        );
        assert!(
            !entries.iter().any(|p| p.contains(".git")),
            ".git should be excluded"
        );
    }

    #[test]
    fn estimate_size_matches_file_sizes() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::write(dir.path().join("b.txt"), b"world!!").unwrap();

        let size = estimate_archive_size(dir.path(), &[]).unwrap();
        assert_eq!(size, 12); // 5 + 7
    }

    #[test]
    fn returns_error_for_non_directory() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("file.txt");
        fs::write(&file, b"x").unwrap();
        assert!(create_source_archive(&file, &[]).is_err());
    }
}
