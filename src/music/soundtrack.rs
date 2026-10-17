//! Native soundtrack decoding and persistent, validated HTTP download caching.

use super::*;
use std::{io::Write, path::Path};

pub(super) type Download = (Soundtrack, Result<Loaded, String>);

pub(super) enum Loaded {
    Audio(AudioSource),
    Module(Arc<Module>),
}

fn decode(bytes: Arc<[u8]>) -> Result<Loaded, String> {
    if bytes.starts_with(b"Extended Module: ") {
        return Module::load_xm(&bytes)
            .map(|module| Loaded::Module(Arc::new(module)))
            .map_err(|error| error.to_string());
    }
    let source = AudioSource { bytes };
    if let Ok(mut decoder) = rodio::Decoder::new(std::io::Cursor::new(source.clone()))
        && decoder.next().is_some()
    {
        return Ok(Loaded::Audio(source));
    }
    Module::load_mod(&source.bytes)
        .map(|module| Loaded::Module(Arc::new(module)))
        .map_err(|error| format!("unsupported or invalid soundtrack: {error}"))
}

pub(super) fn load(source: &Soundtrack, cache: Option<&Path>) -> Result<Loaded, String> {
    let url = match source {
        Soundtrack::File(path) => {
            return decode(
                std::fs::read(path)
                    .map_err(|error| error.to_string())?
                    .into(),
            );
        }
        Soundtrack::Url(url) => url.split('#').next().unwrap(),
    };
    let cached =
        cache.map(|directory| directory.join(blake3::hash(url.as_bytes()).to_hex().as_str()));
    if let Some(path) = &cached
        && let Ok(bytes) = std::fs::read(path)
        && let Ok(track) = decode(bytes.into())
    {
        return Ok(track);
    }
    info!("Downloading soundtrack {url}");
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into();
    let bytes: Arc<[u8]> = agent
        .get(url)
        .call()
        .map_err(|error| error.to_string())?
        .body_mut()
        .with_config()
        .limit(128 * 1024 * 1024)
        .read_to_vec()
        .map_err(|error| error.to_string())?
        .into();
    let track = decode(Arc::clone(&bytes))?;
    if let Some(path) = &cached
        && let Err(error) = store(path, &bytes)
    {
        warn!("Could not cache soundtrack {}: {error}", path.display());
    }
    Ok(track)
}

fn store(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let directory = path.parent().unwrap();
    std::fs::create_dir_all(directory)?;
    let mut file = tempfile::NamedTempFile::new_in(directory)?;
    file.write_all(bytes)?;
    file.persist(path)?;
    Ok(())
}

#[cfg(test)]
#[path = "../tests/soundtrack.rs"]
mod tests;
