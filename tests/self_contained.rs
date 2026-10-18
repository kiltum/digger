//! Guards the project's self-containment rule: the game must build and run
//! from `src/` + `assets/` alone. `ref/` is archival and must never be a build
//! or runtime dependency (and is excluded from packaging).

use std::path::Path;

#[test]
fn assets_are_embedded_and_present() {
    for name in ["vga.spr", "vgatext.spr", "vtitle.bmp"] {
        let path = Path::new("assets").join(name);
        let meta = std::fs::metadata(&path).unwrap_or_else(|e| {
            panic!("missing bundled asset {}: {e}", path.display());
        });
        assert!(meta.len() > 0, "empty bundled asset {}", path.display());
    }
    let assets_rs = std::fs::read_to_string("src/assets.rs").unwrap();
    assert!(assets_rs.contains("include_bytes!(\"../assets/"));
}

#[test]
fn source_does_not_reference_the_archival_tree() {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    let mut files = Vec::new();
    walk(Path::new("src"), &mut files);
    for file in files {
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(
            !text.contains("include_bytes!(\"../ref")
                && !text.contains("include_str!(\"../ref")
                && !text.contains("include_bytes!(\\\"..\\\\/ref"),
            "{} references the archival ref/ tree",
            file.display()
        );
    }
}

#[test]
fn cargo_package_excludes_ref() {
    let manifest = std::fs::read_to_string("Cargo.toml").unwrap();
    assert!(
        manifest.contains("exclude") && manifest.contains("ref"),
        "Cargo.toml must exclude ref/ from packaged releases"
    );
    assert!(
        manifest.contains("digger.sco"),
        "Cargo.toml must exclude the local high-score file"
    );
}
