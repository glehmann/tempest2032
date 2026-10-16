//! Download, validation, atomic caching, offline reuse, and tracker decoding.

use super::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn server(status: &str, bytes: Vec<u8>) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/download?id=42", listener.local_addr().unwrap());
    let status = status.to_owned();
    let task = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        let mut buffer = [0; 1024];
        while !request.windows(4).any(|part| part == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).unwrap();
            assert_ne!(count, 0);
            request.extend_from_slice(&buffer[..count]);
        }
        write!(
            stream,
            "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            bytes.len()
        )
        .unwrap();
        stream.write_all(&bytes).unwrap();
        String::from_utf8(request).unwrap()
    });
    (url, task)
}

fn xm() -> Vec<u8> {
    let mut bytes = vec![0; 336];
    bytes[..17].copy_from_slice(b"Extended Module: ");
    bytes[37] = 0x1a;
    bytes[58..60].copy_from_slice(&0x104_u16.to_le_bytes());
    bytes[60..64].copy_from_slice(&276_u32.to_le_bytes());
    for (offset, value) in [(64, 1_u16), (68, 2), (70, 1), (76, 6), (78, 125)] {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }
    bytes.extend([9, 0, 0, 0, 0, 1, 0, 0, 0]); // One empty pattern row.
    bytes
}

#[test]
fn url_downloads_are_validated_and_reused_offline_without_the_fragment() {
    let cache = tempfile::tempdir().unwrap();
    let (url, task) = server("200 OK", super::super::tests::wav());
    let source = Soundtrack::Url(format!("{url}#music.wav"));
    assert!(matches!(
        load(&source, Some(cache.path())),
        Ok(Loaded::Audio(_))
    ));
    let request = task.join().unwrap();
    assert!(request.starts_with("GET /download?id=42 HTTP/1.1\r\n"));
    assert!(!request.contains("#music.wav"));
    // The listener is gone; this must use the on-disk cache, not the server.
    assert!(matches!(
        load(&Soundtrack::Url(url), Some(cache.path())),
        Ok(Loaded::Audio(_))
    ));
    assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 1);
}

#[test]
fn invalid_downloads_and_http_errors_are_not_cached() {
    for (status, bytes) in [
        ("200 OK", b"<html>not music</html>".to_vec()),
        ("404 Not Found", xm()),
    ] {
        let cache = tempfile::tempdir().unwrap();
        let (url, task) = server(status, bytes);
        assert!(load(&Soundtrack::Url(url), Some(cache.path())).is_err());
        task.join().unwrap();
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
    }
}

#[test]
fn corrupted_cache_is_replaced_and_cache_write_failure_does_not_stop_playback() {
    let directory = tempfile::tempdir().unwrap();
    let (url, task) = server("200 OK", xm());
    let cached = directory
        .path()
        .join(blake3::hash(url.as_bytes()).to_hex().as_str());
    std::fs::write(&cached, b"broken cache").unwrap();
    assert!(matches!(
        load(&Soundtrack::Url(url), Some(directory.path())),
        Ok(Loaded::Module(_))
    ));
    task.join().unwrap();
    assert!(
        std::fs::read(cached)
            .unwrap()
            .starts_with(b"Extended Module: ")
    );

    let blocked = directory.path().join("file-not-directory");
    std::fs::write(&blocked, b"occupied").unwrap();
    let (url, task) = server("200 OK", xm());
    assert!(matches!(
        load(&Soundtrack::Url(url), Some(&blocked)),
        Ok(Loaded::Module(_))
    ));
    task.join().unwrap();
}

#[test]
fn local_mod_and_xm_tracks_stream_with_owned_module_data() {
    for bytes in [DATA[4].to_vec(), xm()] {
        let Loaded::Module(module) = decode(bytes).unwrap() else {
            panic!("expected tracker module")
        };
        let mut decoder = Track(module).decoder();
        assert_eq!(decoder.channels(), 2);
        assert_eq!(decoder.sample_rate(), RATE);
        assert_eq!(
            decoder.by_ref().take(RATE as usize * 4).count(),
            RATE as usize * 4
        );
    }
}

#[test]
#[ignore = "manual check requiring a connection to Mod Archive"]
fn modarchive_demo_downloads_decodes_and_plays_from_cache() {
    let cache = tempfile::tempdir().unwrap();
    let source = Soundtrack::Url(
        "https://api.modarchive.org/downloads.php?moduleid=139997#acid_butcher.xm".into(),
    );
    let Loaded::Module(module) = load(&source, Some(cache.path())).unwrap() else {
        panic!("expected XM module")
    };
    let mut decoder = Track(module).decoder();
    assert!(
        decoder
            .by_ref()
            .take(RATE as usize * 20)
            .any(|sample| sample.unsigned_abs() > 100)
    );
    assert!(matches!(
        load(&source, Some(cache.path())),
        Ok(Loaded::Module(_))
    ));
}
