use super::*;

struct Fixture {
    temp: tempfile::TempDir,
    root: PathBuf,
    source: PathBuf,
}

impl Fixture {
    fn new(ids: &[u32]) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("Steam Démo");
        fs::create_dir_all(root.join("steamapps")).unwrap();
        fs::create_dir_all(root.join("config")).unwrap();
        fs::create_dir_all(root.join("userdata")).unwrap();
        for id in ids {
            fs::create_dir_all(root.join(format!("userdata/{id}/config"))).unwrap();
        }
        let source = temp.path().join("downloaded game");
        fs::write(&source, b"executable version one").unwrap();
        Self { temp, root, source }
    }

    fn shortcuts(&self, user: u32) -> PathBuf {
        self.root
            .join(format!("userdata/{user}/config/shortcuts.vdf"))
    }

    fn destination(&self) -> PathBuf {
        self.temp.path().join("Installed Tempest 2032 Édition")
    }

    fn login(&self, accounts: &[(u32, bool, u64)]) {
        let entries: String = accounts
            .iter()
            .map(|(id, recent, timestamp)| {
                format!(
                    "\"{}\"\n{{\n\"MostRecent\" \"{}\"\n\"Timestamp\" \"{timestamp}\"\n}}\n",
                    STEAM_ID_BASE + u64::from(*id),
                    u8::from(*recent)
                )
            })
            .collect();
        fs::write(
            self.root.join("config/loginusers.vdf"),
            format!("\"users\"\n{{\n{entries}}}\n"),
        )
        .unwrap();
    }
}

fn foreign_shortcuts() -> Vec<u8> {
    // A literal Steam-format record, including UTF-8, a float, uint64, wide
    // string, and duplicate tag keys that a schema-specific writer could lose.
    [
        b"\0shortcuts\0\0".as_slice(),
        b"7\0\x01AppName\0Other Game\0",
        b"\x01Exe\0\"/other game\"\0\x02appid\0\x01\x02\x03\x84",
        b"\x03FutureFloat\0\0\0\x80\x3f\x07Future64\0\x01\x02\x03\x04\x05\x06\x07\x08",
        b"\x05FutureWide\0A\0\0\0\0tags\0\x010\0First\0\x010\0Second\0\x08\x08\x08\x08",
    ]
    .concat()
}

#[test]
fn binary_vdf_roundtrips_unknown_fields_and_rejects_truncation_or_trailing_data() {
    let bytes = foreign_shortcuts();
    assert_eq!(Object::parse(&bytes).unwrap().bytes(), bytes);
    for length in 0..bytes.len() {
        assert!(
            Object::parse(&bytes[..length]).is_err(),
            "accepted truncated length {length}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(Object::parse(&extra).is_err());
    let unsupported = b"\x09unknown\0\x08";
    assert!(Object::parse(unsupported).is_err());
    let alternate = b"\0shortcuts\0\x0b\x0b";
    assert_eq!(Object::parse(alternate).unwrap().bytes(), alternate);
}

#[test]
fn platform_locations_and_executable_names_cover_native_and_sandboxed_steam() {
    let data = Path::new("user-data");
    let native = Path::new("native/Steam");
    assert_eq!(
        install_directory(Platform::Linux, data, native),
        data.join("tempest2032")
    );
    for platform in [Platform::Windows, Platform::MacOs] {
        assert_eq!(
            install_directory(platform, data, native),
            data.join("Tempest 2032")
        );
    }
    let flatpak = Path::new("home/.var/app/com.valvesoftware.Steam/.local/share/Steam");
    let snap = Path::new("home/snap/steam/common/.local/share/Steam");
    for root in [flatpak, snap] {
        assert_eq!(
            install_directory(Platform::Linux, data, root),
            root.join("tempest2032")
        );
    }
    assert_eq!(Platform::Windows.binary_name(), "tempest2032.exe");
    assert_eq!(Platform::MacOs.binary_name(), "tempest2032");
    assert_eq!(Platform::Linux.binary_name(), "tempest2032");
    let crossover = Path::new(
        "Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files/Steam",
    );
    assert!(compatible_root(Platform::MacOs, native));
    assert!(!compatible_root(Platform::MacOs, crossover));
    assert!(compatible_root(Platform::Windows, crossover));
    assert!(same_exe(
        r#""C:\Users\Ada\Tempest 2032\tempest2032.exe""#,
        "c:/users/ada/tempest 2032/TEMPEST2032.EXE",
        Platform::Windows
    ));
    assert!(!same_exe(
        "/Game/tempest2032",
        "/game/tempest2032",
        Platform::Linux
    ));
}

#[test]
fn most_recent_steam_account_wins_and_ambiguities_have_explicit_overrides() {
    let fixture = Fixture::new(&[42, 99]);
    assert!(
        choose_user(&fixture.root, None)
            .unwrap_err()
            .to_string()
            .contains("--steam-user")
    );
    fixture.login(&[(42, false, 900), (99, true, 100)]);
    assert_eq!(choose_user(&fixture.root, None).unwrap(), 99);
    assert_eq!(choose_user(&fixture.root, Some(42)).unwrap(), 42);
    assert!(choose_user(&fixture.root, Some(13)).is_err());
    fixture.login(&[(42, false, 900), (99, false, 100)]);
    assert_eq!(choose_user(&fixture.root, None).unwrap(), 42);
    fixture.login(&[(42, false, 900), (99, false, 900)]);
    assert!(choose_user(&fixture.root, None).is_err());
    fixture.login(&[(13, true, 1000)]);
    assert!(choose_user(&fixture.root, None).is_err());
    assert_eq!(choose_user(&fixture.root, Some(42)).unwrap(), 42);
}

#[test]
fn installation_selection_uses_login_recency_and_deduplicates_paths() {
    let a = Fixture::new(&[42, 99]);
    let b = Fixture::new(&[42]);
    a.login(&[(42, false, 10), (99, true, 1000)]);
    b.login(&[(42, true, 20)]);
    assert_eq!(
        choose_root(vec![a.root.clone(), a.root.clone()], None).unwrap(),
        fs::canonicalize(&a.root).unwrap()
    );
    let stale = a.temp.path().join("Uninstalled Steam");
    fs::create_dir_all(stale.join("steamapps")).unwrap();
    assert_eq!(
        choose_root(vec![stale.clone(), a.root.clone()], None).unwrap(),
        fs::canonicalize(&a.root).unwrap()
    );
    assert!(steam_root(&stale).is_err());
    assert_eq!(
        choose_root(vec![a.root.clone(), b.root.clone()], Some(42)).unwrap(),
        fs::canonicalize(&b.root).unwrap()
    );
    assert_eq!(
        choose_root(vec![a.root.clone(), b.root.clone()], None).unwrap(),
        fs::canonicalize(&a.root).unwrap()
    );
    b.login(&[(42, true, 1000)]);
    assert!(
        choose_root(vec![a.root, b.root], None)
            .unwrap_err()
            .to_string()
            .contains("--steam-dir")
    );
    assert!(
        choose_root(Vec::new(), None)
            .unwrap_err()
            .to_string()
            .contains("not found")
    );
}

#[test]
fn invalid_or_empty_steam_profiles_fail_without_installing_a_binary() {
    let empty = Fixture::new(&[]);
    assert!(choose_user(&empty.root, None).is_err());
    let fixture = Fixture::new(&[42]);
    fs::write(
        fixture.root.join("config/loginusers.vdf"),
        b"users { broken",
    )
    .unwrap();
    assert!(choose_user(&fixture.root, None).is_err());
    assert_eq!(choose_user(&fixture.root, Some(42)).unwrap(), 42);
    fs::write(fixture.shortcuts(42), b"corrupt shortcuts").unwrap();
    let error = install(
        &fixture.root,
        42,
        &fixture.source,
        &fixture.destination(),
        Platform::Linux,
    )
    .unwrap_err();
    assert!(error.to_string().contains("shortcuts.vdf"));
    assert_eq!(
        fs::read(fixture.shortcuts(42)).unwrap(),
        b"corrupt shortcuts"
    );
    assert!(!fixture.destination().exists());
}

#[test]
fn install_copies_the_binary_and_preserves_all_existing_shortcut_fields() {
    for platform in [Platform::Linux, Platform::Windows, Platform::MacOs] {
        let fixture = Fixture::new(&[42]);
        let old = foreign_shortcuts();
        fs::write(fixture.shortcuts(42), &old).unwrap();
        let installed = install(
            &fixture.root,
            42,
            &fixture.source,
            &fixture.destination(),
            platform,
        )
        .unwrap();
        assert_eq!(
            fs::read(&installed.binary).unwrap(),
            fs::read(&fixture.source).unwrap()
        );
        assert_eq!(fs::read(installed.backup.unwrap()).unwrap(), old);
        let document = Object::parse(&fs::read(fixture.shortcuts(42)).unwrap()).unwrap();
        let shortcuts = document.get("shortcuts").unwrap().as_object().unwrap();
        let original = Object::parse(&old).unwrap();
        assert_eq!(
            shortcuts.fields[0],
            original
                .get("shortcuts")
                .unwrap()
                .as_object()
                .unwrap()
                .fields[0]
        );
        assert_eq!(shortcuts.fields.len(), 2);
        let game = shortcuts.get("8").unwrap().as_object().unwrap();
        assert_eq!(
            game.get("Exe").unwrap().as_str().unwrap(),
            quoted(&installed.binary).unwrap()
        );
        assert_eq!(
            game.get("StartDir").unwrap().as_str().unwrap(),
            quoted(&fixture.destination()).unwrap()
        );
        assert_eq!(
            game.get("LaunchOptions").unwrap().as_str(),
            Some("--fullscreen")
        );
        assert_eq!(game.get("appid").unwrap().as_u32(), Some(installed.app_id));
        assert_eq!(game.get("AppName").unwrap().as_str(), Some("Tempest 2032"));
        assert_ne!(installed.app_id & 0x8000_0000, 0);
        for flag in ["AllowOverlay", "AllowDesktopConfig"] {
            assert_eq!(game.get(flag).unwrap().as_u32(), Some(1));
        }
    }
}

#[test]
fn reinstall_updates_the_binary_without_duplicates_or_losing_custom_settings() {
    let fixture = Fixture::new(&[42]);
    let directory = fixture.destination();
    let first = install(
        &fixture.root,
        42,
        &fixture.source,
        &directory,
        Platform::Linux,
    )
    .unwrap();
    assert!(first.backup.is_none());
    let mut document = Object::parse(&fs::read(fixture.shortcuts(42)).unwrap()).unwrap();
    let game = document
        .get_mut("shortcuts")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut("0")
        .unwrap()
        .as_object_mut()
        .unwrap();
    for (key, value) in [
        ("LaunchOptions", "--volume 0.2 --level 2"),
        ("AppName", "My Tempest 2032"),
        ("icon", "custom icon.png"),
        ("StartDir", "custom levels directory"),
        ("FutureSetting", "preserve me"),
    ] {
        game.set(Field::string(key, value));
    }
    let custom = document.bytes();
    fs::write(fixture.shortcuts(42), &custom).unwrap();
    fs::write(&fixture.source, b"updated executable").unwrap();
    let second = install(
        &fixture.root,
        42,
        &fixture.source,
        &directory,
        Platform::Linux,
    )
    .unwrap();
    assert_eq!(second.app_id, first.app_id);
    assert!(second.backup.is_none());
    assert_eq!(fs::read(fixture.shortcuts(42)).unwrap(), custom);
    assert_eq!(fs::read(&second.binary).unwrap(), b"updated executable");
    let self_install = install(
        &fixture.root,
        42,
        &second.binary,
        &directory,
        Platform::Linux,
    )
    .unwrap();
    assert_eq!(self_install.app_id, second.app_id);
    assert_eq!(
        fs::read(self_install.binary).unwrap(),
        b"updated executable"
    );
}

#[test]
fn backups_are_never_overwritten_and_duplicate_shortcuts_are_not_silently_removed() {
    let fixture = Fixture::new(&[42]);
    let directory = fixture.destination();
    let old = foreign_shortcuts();
    fs::write(fixture.shortcuts(42), &old).unwrap();
    let first = install(
        &fixture.root,
        42,
        &fixture.source,
        &directory,
        Platform::Linux,
    )
    .unwrap();
    fs::write(fixture.shortcuts(42), &old).unwrap();
    let second = install(
        &fixture.root,
        42,
        &fixture.source,
        &directory,
        Platform::Linux,
    )
    .unwrap();
    assert_ne!(first.backup, second.backup);
    assert_eq!(fs::read(first.backup.unwrap()).unwrap(), old);
    let bytes = fs::read(fixture.shortcuts(42)).unwrap();
    let mut document = Object::parse(&bytes).unwrap();
    let shortcuts = document
        .get_mut("shortcuts")
        .unwrap()
        .as_object_mut()
        .unwrap();
    let duplicate = shortcuts.fields[1].as_object().unwrap().clone();
    shortcuts.fields.push(Field::object("9", duplicate));
    let duplicate_bytes = document.bytes();
    fs::write(fixture.shortcuts(42), &duplicate_bytes).unwrap();
    assert!(
        install(
            &fixture.root,
            42,
            &fixture.source,
            &directory,
            Platform::Linux
        )
        .is_err()
    );
    assert_eq!(fs::read(fixture.shortcuts(42)).unwrap(), duplicate_bytes);
}

#[test]
fn shortcut_writes_are_locked_and_path_quoting_is_unambiguous() {
    let fixture = Fixture::new(&[42]);
    let path = fixture
        .shortcuts(42)
        .with_file_name("shortcuts.vdf.tempest2032.lock");
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    lock.try_lock().unwrap();
    assert!(
        install(
            &fixture.root,
            42,
            &fixture.source,
            &fixture.destination(),
            Platform::Linux
        )
        .is_err()
    );
    assert!(!fixture.destination().exists());
    assert_eq!(
        quoted(Path::new("folder with spaces/Édition/tempest2032")).unwrap(),
        "\"folder with spaces/Édition/tempest2032\""
    );
    assert!(quoted(Path::new("bad\"name/tempest2032")).is_err());
    assert!(quoted(Path::new("bad\nname/tempest2032")).is_err());
    for name in ["steam", "STEAM.EXE", "steam_osx"] {
        assert!(steam_name(name.as_ref()));
    }
    for name in ["steamwebhelper", "steam_helper", "tempest2032"] {
        assert!(!steam_name(name.as_ref()));
    }
    assert!(!is_thread(sysinfo::get_current_pid().unwrap()));
}

#[cfg(unix)]
#[test]
fn copied_game_keeps_its_unix_executable_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new(&[42]);
    fs::set_permissions(&fixture.source, fs::Permissions::from_mode(0o751)).unwrap();
    let installed = install(
        &fixture.root,
        42,
        &fixture.source,
        &fixture.destination(),
        Platform::Linux,
    )
    .unwrap();
    assert_eq!(
        fs::metadata(installed.binary).unwrap().permissions().mode() & 0o777,
        0o751
    );
}
