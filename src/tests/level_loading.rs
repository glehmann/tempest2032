//! Embedded fallback, partial disk overrides, and invalid external catalogs.

use super::*;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "tempest2032-level-loading-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, source: &str) {
        std::fs::write(self.0.join(name), source).unwrap();
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

const FIRST: &str = include_str!("../../levels/01.toml");

#[test]
fn missing_and_empty_directories_load_the_full_embedded_catalog() {
    let directory = Directory::new();
    directory.write("README.txt", "External level overrides are optional.");
    for path in [&directory.0, &directory.0.join("missing")] {
        let levels = Levels::load(path).unwrap();
        assert_eq!(levels.0.len(), BUNDLED_LEVELS.len());
        assert_eq!(
            levels
                .0
                .iter()
                .map(|level| level.number)
                .collect::<Vec<_>>(),
            (1..=BUNDLED_LEVELS.len()).collect::<Vec<_>>()
        );
        assert_eq!(levels.0[0].flippers.count, 25);
        assert_eq!(levels.0[2].web.lanes(), 11);
        assert_eq!(levels.0[99].number, 100);
        assert!(!levels.0[99].flippers.enabled);
    }
}

#[test]
fn external_overrides_win_by_level_number_and_missing_levels_fall_back() {
    for name in ["01.toml", "custom.toml"] {
        let directory = Directory::new();
        directory.write(
            name,
            &FIRST
                .replace("count = 25", "count = 3")
                .replace("depth = 160", "depth = 200"),
        );
        let levels = Levels::load(&directory.0).unwrap();
        assert_eq!(levels.0.len(), BUNDLED_LEVELS.len());
        assert_eq!(levels.0[0].flippers.count, 3);
        assert_eq!(levels.0[0].web.depth, 200);
        assert_eq!(levels.0[1].flippers.count, 25);
        assert_eq!(levels.0[1].flippers.first_spawn_ticks, 51);
    }
}

#[test]
fn existing_malformed_or_unreadable_levels_are_errors() {
    let directory = Directory::new();
    directory.write("01.toml", "number =");
    let error = Levels::load(&directory.0).err().unwrap();
    assert!(error.contains("01.toml"));
    std::fs::remove_file(directory.0.join("01.toml")).unwrap();
    std::fs::create_dir(directory.0.join("01.toml")).unwrap();
    let error = Levels::load(&directory.0).err().unwrap();
    assert!(error.contains("01.toml"));
}

#[test]
fn duplicate_override_numbers_still_fail_validation() {
    let directory = Directory::new();
    directory.write("01.toml", FIRST);
    directory.write("duplicate.toml", FIRST);
    let error = Levels::load(&directory.0).err().unwrap();
    assert!(error.contains("expected level number 2, found 1"));
}

#[test]
fn extra_levels_can_extend_the_catalog_but_cannot_leave_a_gap() {
    let directory = Directory::new();
    let next = BUNDLED_LEVELS.len() + 1;
    directory.write(
        "extra.toml",
        &FIRST.replace("number = 1", &format!("number = {next}")),
    );
    let levels = Levels::load(&directory.0).unwrap();
    assert_eq!(levels.0.len(), next);
    assert_eq!(levels.0.last().unwrap().number, next);
    directory.write(
        "extra.toml",
        &FIRST.replace("number = 1", &format!("number = {}", next + 1)),
    );
    let error = Levels::load(&directory.0).err().unwrap();
    assert!(error.contains(&format!("expected level number {next}")));
}

#[test]
fn an_existing_file_instead_of_a_directory_is_an_error() {
    let directory = Directory::new();
    directory.write("catalog", "not a directory");
    assert!(Levels::load(&directory.0.join("catalog")).is_err());
}

#[test]
fn soundtrack_paths_are_relative_to_the_level_file_and_need_not_exist() {
    let directory = Directory::new();
    for path in [
        PathBuf::from("music/song.flac"),
        directory.0.join("absolute.mp3"),
    ] {
        let setting = toml::Value::String(path.to_str().unwrap().to_owned());
        directory.write("01.toml", &format!("soundtrack = {setting}\n{FIRST}"));
        let levels = Levels::load(&directory.0).unwrap();
        assert_eq!(levels.0[0].soundtrack, Some(directory.0.join(path)));
        assert!(levels.0[1].soundtrack.is_none());
        assert!(!levels.is_standard());
    }
    assert!(Level::parse(&format!("soundtrack = \"\"\n{FIRST}"), "empty").is_err());
}

#[test]
fn demo_is_a_valid_partial_override() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/custom-level");
    let levels = Levels::load(&directory).unwrap();
    assert_eq!(levels.0.len(), BUNDLED_LEVELS.len());
    assert_eq!(levels.0[0].web.lanes(), 8);
    assert_eq!(
        levels.0[0].soundtrack,
        Some(directory.join("soundtrack.ogg"))
    );
    assert!(levels.0[1..] == Levels::load(&directory.join("missing")).unwrap().0[1..]);
}
