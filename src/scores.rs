//! Local difficulty settings and stable, per-difficulty score rankings.

use crate::difficulty::Difficulty;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::{io::Write, path::PathBuf};

pub(crate) const LIMIT: usize = 10;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Entry {
    pub(crate) initials: String,
    pub(crate) score: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Data {
    version: u8,
    pub(crate) difficulty: Difficulty,
    pub(crate) initials: String,
    pub(crate) tables: [Vec<Entry>; 3],
}

impl Default for Data {
    fn default() -> Self {
        Self {
            version: 1,
            difficulty: Difficulty::Normal,
            initials: "AAA".into(),
            tables: default(),
        }
    }
}

impl Data {
    fn parse(text: &str) -> Result<Self, String> {
        let mut data: Self = toml::from_str(text).map_err(|error| error.to_string())?;
        if data.version != 1
            || !valid_initials(&data.initials)
            || data
                .tables
                .iter()
                .flatten()
                .any(|entry| !valid_initials(&entry.initials))
        {
            return Err("Unsupported or invalid player data".into());
        }
        for table in &mut data.tables {
            table.sort_by_key(|entry| std::cmp::Reverse(entry.score));
            table.truncate(LIMIT);
        }
        Ok(data)
    }

    pub(crate) fn rank(&self, difficulty: Difficulty, score: u32) -> Option<usize> {
        if score == 0 {
            return None;
        }
        let table = &self.tables[difficulty.index()];
        let rank = table
            .iter()
            .position(|entry| score > entry.score)
            .unwrap_or(table.len());
        (rank < LIMIT).then_some(rank)
    }

    pub(crate) fn insert(
        &mut self,
        difficulty: Difficulty,
        score: u32,
        initials: String,
    ) -> Option<usize> {
        let rank = self.rank(difficulty, score)?;
        self.initials.clone_from(&initials);
        let table = &mut self.tables[difficulty.index()];
        table.insert(rank, Entry { initials, score });
        table.truncate(LIMIT);
        Some(rank)
    }
}

#[derive(Resource)]
pub(crate) struct Store {
    pub(crate) data: Data,
    #[cfg(not(target_arch = "wasm32"))]
    path: Option<PathBuf>,
    #[cfg(target_arch = "wasm32")]
    writable: bool,
    pub(crate) notice: String,
}

impl Store {
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn load() -> Self {
        Self::open(dirs::data_local_dir().map(|path| path.join("tempest2032/player.toml")))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn open(path: Option<PathBuf>) -> Self {
        let mut store = Self {
            data: default(),
            path,
            notice: String::new(),
        };
        let result = (|| {
            let path = store
                .path
                .as_ref()
                .ok_or("Could not locate the application-data directory".to_string())?;
            let text = match std::fs::read_to_string(path) {
                Ok(text) => text,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Ok(Data::default());
                }
                Err(error) => return Err(error.to_string()),
            };
            Data::parse(&text)
        })();
        match result {
            Ok(data) => store.data = data,
            Err(error) => {
                store.notice = format!(
                    "Could not load player data: {error}. Saving is disabled for this session."
                );
                store.path = None;
            }
        }
        store
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn save(&mut self) {
        let Some(path) = &self.path else { return };
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let directory = path.parent().unwrap();
            std::fs::create_dir_all(directory)?;
            let mut temporary = tempfile::NamedTempFile::new_in(directory)?;
            temporary.write_all(toml::to_string(&self.data)?.as_bytes())?;
            temporary.as_file().sync_all()?;
            temporary.persist(path)?;
            Ok(())
        })();
        self.notice = result.err().map_or_else(String::new, |error| {
            format!("Could not save player data: {error}. Scores remain available until you quit.")
        });
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn load() -> Self {
        let result = (|| match storage()?
            .get_item("tempest2032/player.toml")
            .map_err(|error| format!("{error:?}"))?
        {
            Some(text) => Data::parse(&text),
            None => Ok(Data::default()),
        })();
        match result {
            Ok(data) => Self {
                data,
                writable: true,
                notice: String::new(),
            },
            Err(error) => Self {
                data: default(),
                writable: false,
                notice: format!(
                    "Could not load player data: {error}. Saving is disabled for this session."
                ),
            },
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn save(&mut self) {
        if !self.writable {
            return;
        }
        let result = (|| {
            storage()?
                .set_item(
                    "tempest2032/player.toml",
                    &toml::to_string(&self.data).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("{error:?}"))
        })();
        self.notice = result.err().map_or_else(String::new, |error| {
            format!(
                "Could not save player data: {error}. Scores remain available until you reload."
            )
        });
    }
}

#[cfg(target_arch = "wasm32")]
fn storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .unwrap()
        .local_storage()
        .map_err(|error| format!("{error:?}"))?
        .ok_or_else(|| "Browser storage is unavailable".into())
}

fn valid_initials(value: &str) -> bool {
    value.len() == 3 && value.bytes().all(|letter| letter.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rankings_are_separate_stable_on_ties_and_bounded() {
        let mut data = Data::default();
        assert_eq!(data.rank(Difficulty::Normal, 0), None);
        for score in (1..=10).rev() {
            data.insert(Difficulty::Normal, score, "AAA".into());
        }
        assert_eq!(data.rank(Difficulty::Normal, 1), None);
        assert_eq!(data.insert(Difficulty::Normal, 10, "BBB".into()), Some(1));
        assert_eq!(data.tables[1][0].initials, "AAA");
        assert_eq!(data.tables[1].len(), LIMIT);
        assert_eq!(data.rank(Difficulty::High, 1), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn round_trip_and_invalid_files_are_not_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("player.toml");
        let mut store = Store::open(Some(path.clone()));
        store.data.difficulty = Difficulty::High;
        store.data.insert(Difficulty::Low, 123, "XYZ".into());
        store.save();
        assert!(store.notice.is_empty());
        let loaded = Store::open(Some(path.clone()));
        assert_eq!(loaded.data.difficulty, Difficulty::High);
        assert_eq!(loaded.data.initials, "XYZ");
        assert_eq!(loaded.data.tables[0][0].score, 123);
        std::fs::write(&path, "invalid [").unwrap();
        let mut broken = Store::open(Some(path.clone()));
        assert!(!broken.notice.is_empty());
        broken.save();
        assert_eq!(std::fs::read_to_string(path).unwrap(), "invalid [");
    }
}
