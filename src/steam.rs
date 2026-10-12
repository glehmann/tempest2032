//! Per-user executable installation and non-Steam shortcut registration.

mod vdf;

use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
use vdf::{Field, Object};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const STEAM_ID_BASE: u64 = 76_561_197_960_265_728;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Platform {
    Linux,
    Windows,
    MacOs,
}

impl Platform {
    fn current() -> Result<Self> {
        if cfg!(target_os = "windows") {
            Ok(Self::Windows)
        } else if cfg!(target_os = "macos") {
            Ok(Self::MacOs)
        } else if cfg!(target_os = "linux") {
            Ok(Self::Linux)
        } else {
            Err("Steam installation is supported on Linux, Windows, and macOS".into())
        }
    }

    fn binary_name(self) -> &'static str {
        if self == Self::Windows {
            "tempest2032.exe"
        } else {
            "tempest2032"
        }
    }
}

pub(crate) fn run(directory: Option<&Path>, user: Option<u32>) -> Result<()> {
    let platform = Platform::current()?;
    let roots = match directory {
        Some(path) => {
            let root = steam_root(path)?;
            if !compatible_root(platform, &root) {
                return Err("This Steam directory belongs to a Windows client; register it with the Windows build of Tempest 2032".into());
            }
            vec![root]
        }
        None => steamlocate::locate_all()
            .map_err(|e| {
                format!(
                    "Could not locate Steam: {e}. Use --steam-dir PATH for a custom installation"
                )
            })?
            .into_iter()
            .filter(|s| compatible_root(platform, s.path()))
            .map(|s| s.path().to_path_buf())
            .collect(),
    };
    let root = choose_root(roots, user)?;
    let user = choose_user(&root, user)?;
    warn_if_steam_running();
    let data =
        dirs::data_local_dir().ok_or("Could not locate the user's application-data directory")?;
    let destination = install_directory(platform, &data, &root);
    let executable = std::env::current_exe()?;
    let installed = install(&root, user, &executable, &destination, platform)?;
    println!("Installed Tempest 2032: {}", installed.binary.display());
    println!("Steam account: {user} ({})", root.display());
    println!("Shortcut ID: {}", installed.app_id);
    if let Some(backup) = installed.backup {
        println!("Shortcut backup: {}", backup.display());
    }
    println!("Start Steam to find Tempest 2032 in your library.");
    Ok(())
}

fn install_directory(platform: Platform, data: &Path, steam: &Path) -> PathBuf {
    let components: Vec<_> = steam.components().map(|c| c.as_os_str()).collect();
    let sandboxed = components.iter().any(|c| *c == "com.valvesoftware.Steam")
        || components
            .windows(2)
            .any(|c| c[0] == "snap" && c[1] == "steam");
    if platform == Platform::Linux && sandboxed {
        steam.join("tempest2032")
    } else {
        data.join(if platform == Platform::Linux {
            "tempest2032"
        } else {
            "Tempest 2032"
        })
    }
}

fn compatible_root(platform: Platform, root: &Path) -> bool {
    // steamlocate also discovers Windows Steam clients inside CrossOver bottles.
    platform == Platform::Windows || !root.components().any(|c| c.as_os_str() == "drive_c")
}

fn steam_root(path: &Path) -> Result<PathBuf> {
    let root = fs::canonicalize(path)
        .map_err(|e| format!("Could not open Steam directory {}: {e}", path.display()))?;
    if !root.join("steamapps").is_dir() || !root.join("userdata").is_dir() {
        return Err(format!(
            "{} is not a Steam installation with userdata; sign into Steam at least once",
            root.display()
        )
        .into());
    }
    Ok(root)
}

fn choose_root(roots: Vec<PathBuf>, user: Option<u32>) -> Result<PathBuf> {
    let mut unique = BTreeSet::new();
    for root in roots {
        let Ok(root) = steam_root(&root) else {
            continue;
        };
        if user.is_none_or(|id| root.join("userdata").join(id.to_string()).is_dir()) {
            unique.insert(root);
        }
    }
    if unique.is_empty() {
        return Err("Steam installation not found for this user. Install Steam and sign in, or provide --steam-dir PATH".into());
    }
    if unique.len() == 1 {
        return Ok(unique.into_iter().next().unwrap());
    }
    let mut ranked = Vec::new();
    for root in &unique {
        let accounts = account_info(root)?;
        let latest = if let Some(id) = user {
            accounts.get(&id).copied()
        } else {
            accounts.values().copied().max()
        }
        .unwrap_or_default();
        ranked.push((latest, root));
    }
    ranked.sort_by_key(|(rank, _)| *rank);
    let last = ranked.last().unwrap();
    if last.0 != (false, 0) && ranked[ranked.len() - 2].0 != last.0 {
        return Ok(last.1.clone());
    }
    Err(format!(
        "Multiple Steam installations found: {}. Select one with --steam-dir PATH",
        unique
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
    .into())
}

fn account_ids(root: &Path) -> Result<BTreeSet<u32>> {
    let mut ids = BTreeSet::new();
    for entry in fs::read_dir(root.join("userdata"))? {
        let entry = entry?;
        if entry.path().is_dir()
            && let Some(id) = entry
                .file_name()
                .to_str()
                .and_then(|s| s.parse::<u32>().ok())
            && id != 0
        {
            ids.insert(id);
        }
    }
    Ok(ids)
}

fn account_info(root: &Path) -> Result<BTreeMap<u32, (bool, u64)>> {
    let ids = account_ids(root)?;
    let mut accounts: BTreeMap<_, _> = ids.iter().map(|&id| (id, (false, 0))).collect();
    let path = root.join("config/loginusers.vdf");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(accounts),
        Err(e) => return Err(format!("Could not read {}: {e}", path.display()).into()),
    };
    let login =
        keyvalues_parser::parse(&text).map_err(|_| format!("Invalid {}", path.display()))?;
    let users = login
        .value
        .get_obj()
        .filter(|_| login.key.eq_ignore_ascii_case("users"))
        .ok_or("Steam loginusers.vdf does not contain a users object")?;
    for (steam_id, values) in users.iter() {
        let Some(id) = steam_id
            .parse::<u64>()
            .ok()
            .and_then(|v| v.checked_sub(STEAM_ID_BASE))
            .and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        let info = values
            .first()
            .and_then(|v| v.get_obj())
            .ok_or("Invalid Steam login account")?;
        let field = |key: &str| {
            info.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .and_then(|(_, v)| v.first())
                .and_then(|v| v.get_str())
        };
        let recent = field("MostRecent") == Some("1");
        if recent && !ids.contains(&id) {
            return Err(format!("Most recent Steam account {id} has no userdata directory; sign into that account first").into());
        }
        if ids.contains(&id) {
            accounts.insert(
                id,
                (
                    recent,
                    field("Timestamp").and_then(|s| s.parse().ok()).unwrap_or(0),
                ),
            );
        }
    }
    Ok(accounts)
}

fn choose_user(root: &Path, user: Option<u32>) -> Result<u32> {
    if let Some(id) = user {
        return if account_ids(root)?.contains(&id) {
            Ok(id)
        } else {
            Err(format!(
                "Steam account {id} has no userdata directory in {}",
                root.display()
            )
            .into())
        };
    }
    let accounts = account_info(root)?;
    if accounts.is_empty() {
        return Err("No Steam accounts found; sign into Steam at least once".into());
    }
    let best = *accounts.values().max().unwrap();
    let matches: Vec<_> = accounts
        .iter()
        .filter(|(_, rank)| **rank == best)
        .map(|(&id, _)| id)
        .collect();
    if matches.len() == 1 {
        return Ok(matches[0]);
    }
    Err(format!("Could not determine the most recently used Steam account. Accounts: {}. Select one with --steam-user ACCOUNT_ID",
        accounts.keys().map(u32::to_string).collect::<Vec<_>>().join(", ")).into())
}

fn steam_name(name: &std::ffi::OsStr) -> bool {
    matches!(
        name.to_string_lossy().to_ascii_lowercase().as_str(),
        "steam" | "steam.exe" | "steam_osx"
    )
}

fn steam_process(process: &sysinfo::Process) -> bool {
    let exe = process.exe().and_then(|exe| exe.file_name());
    let launched = process
        .cmd()
        .first()
        .and_then(|cmd| Path::new(cmd).file_name());
    steam_name(process.name()) || exe.is_some_and(steam_name) || launched.is_some_and(steam_name)
}

/// sysinfo lists every thread as its own process on Linux, and Steam names
/// dozens of them after its own binary.
#[cfg(target_os = "linux")]
fn is_thread(pid: sysinfo::Pid) -> bool {
    fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find_map(|line| line.strip_prefix("Tgid:"))
                .map(|group| group.trim() != pid.to_string())
        })
        .unwrap_or(false)
}

#[cfg(not(target_os = "linux"))]
fn is_thread(_pid: sysinfo::Pid) -> bool {
    false
}
fn warn_if_steam_running() {
    use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};
    let system = System::new_with_specifics(
        RefreshKind::nothing().with_processes(
            ProcessRefreshKind::nothing()
                .with_user(UpdateKind::Always)
                .with_exe(UpdateKind::Always)
                .with_cmd(UpdateKind::Always),
        ),
    );
    let me = sysinfo::get_current_pid()
        .ok()
        .and_then(|pid| system.process(pid))
        .and_then(|p| p.user_id());
    let mut running: Vec<String> = system
        .processes()
        .values()
        .filter(|p| {
            !is_thread(p.pid()) && steam_process(p) && me.is_none_or(|u| p.user_id() == Some(u))
        })
        .map(|p| {
            let exe = p
                .exe()
                .map_or_else(String::new, |exe| format!(" ({})", exe.display()));
            format!("pid {} {}{}", p.pid(), p.name().to_string_lossy(), exe)
        })
        .collect();
    running.sort();
    if !running.is_empty() {
        eprintln!(
            "Warning: Steam is still running ({}). Tempest 2032 shows up in your library after you restart Steam; until then Steam may drop the shortcut again.",
            running.join(", ")
        );
    }
}
#[derive(Debug)]
struct Installed {
    binary: PathBuf,
    app_id: u32,
    backup: Option<PathBuf>,
}

fn install(
    root: &Path,
    user: u32,
    source: &Path,
    directory: &Path,
    platform: Platform,
) -> Result<Installed> {
    if !account_ids(root)?.contains(&user) {
        return Err(format!("Steam account {user} not found").into());
    }
    let binary = directory.join(platform.binary_name());
    let config = root.join("userdata").join(user.to_string()).join("config");
    fs::create_dir_all(&config)?;
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(config.join("shortcuts.vdf.tempest2032.lock"))?;
    lock.try_lock()
        .map_err(|e| format!("Another Steam installer may be running: {e}"))?;
    let path = config.join("shortcuts.vdf");
    let old = match fs::read(&path) {
        Ok(bytes) => Some(bytes),
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(format!("Could not read {}: {e}", path.display()).into()),
    };
    let (bytes, app_id) = shortcut_bytes(old.as_deref(), &binary, platform)
        .map_err(|e| format!("Could not update {}: {e}", path.display()))?;
    fs::create_dir_all(directory)?;
    let source = fs::canonicalize(source)?;
    if fs::canonicalize(&binary).ok().as_ref() != Some(&source) {
        let temporary = NamedTempFile::new_in(directory)?;
        fs::copy(&source, temporary.path())
            .map_err(|e| format!("Could not copy Tempest 2032 to {}: {e}", binary.display()))?;
        temporary.as_file().sync_all()?;
        temporary.persist(&binary).map_err(|e| {
            format!(
                "Could not replace {}: {e}. Close any running Tempest 2032 instance",
                binary.display()
            )
        })?;
    }
    install_artwork(&config, app_id, directory)?;
    let backup = if old.as_deref() == Some(bytes.as_slice()) {
        None
    } else {
        let backup = old
            .as_deref()
            .map(|bytes| backup_shortcuts(&path, bytes))
            .transpose()?;
        let mut temporary = NamedTempFile::new_in(&config)?;
        temporary.write_all(&bytes)?;
        if let Ok(metadata) = fs::metadata(&path) {
            temporary
                .as_file()
                .set_permissions(metadata.permissions())?;
        }
        temporary.as_file().sync_all()?;
        temporary
            .persist(&path)
            .map_err(|e| format!("Could not replace {}: {e}", path.display()))?;
        backup
    };
    Ok(Installed {
        binary,
        app_id,
        backup,
    })
}

fn install_artwork(config: &Path, app_id: u32, directory: &Path) -> Result<()> {
    let grid = config.join("grid");
    fs::create_dir_all(&grid)?;
    let icon = directory.join("icon.png");
    write_missing_artwork(&icon, include_bytes!("../assets/steam/icon.png"))?;
    for (suffix, bytes) in [
        (
            "",
            include_bytes!("../assets/steam/landscape.png").as_slice(),
        ),
        (
            "p",
            include_bytes!("../assets/steam/portrait.png").as_slice(),
        ),
        (
            "_hero",
            include_bytes!("../assets/steam/hero.png").as_slice(),
        ),
        (
            "_logo",
            include_bytes!("../assets/steam/logo.png").as_slice(),
        ),
    ] {
        let stem = format!("{app_id}{suffix}");
        let png = grid.join(format!("{stem}.png"));
        if suffix == "_hero"
            && png.exists()
            && fs::read(&png)? == include_bytes!("../assets/steam/gameplay.png")
        {
            let mut temporary = NamedTempFile::new_in(&grid)?;
            temporary.write_all(bytes)?;
            temporary.as_file().sync_all()?;
            temporary
                .persist(&png)
                .map_err(|e| format!("Could not update Steam artwork {}: {e}", png.display()))?;
        }
        if !["png", "jpg", "jpeg", "webp"]
            .iter()
            .any(|extension| grid.join(format!("{stem}.{extension}")).exists())
        {
            write_missing_artwork(&png, bytes)?;
        }
    }
    Ok(())
}

fn write_missing_artwork(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    let mut temporary = NamedTempFile::new_in(path.parent().unwrap())?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    match temporary.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(e) if e.error.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(format!("Could not install Steam artwork {}: {e}", path.display()).into()),
    }
}

fn backup_shortcuts(path: &Path, bytes: &[u8]) -> Result<PathBuf> {
    for number in 0.. {
        let suffix = if number == 0 {
            String::new()
        } else {
            format!(".{number}")
        };
        let backup = path.with_file_name(format!("shortcuts.vdf.tempest2032-backup{suffix}"));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
        {
            Ok(mut file) => {
                file.write_all(bytes)?;
                file.sync_all()?;
                return Ok(backup);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

fn quoted(path: &Path) -> Result<String> {
    let value = path
        .to_str()
        .ok_or("Steam shortcut paths must be valid Unicode")?;
    if value.contains(['"', '\0', '\n', '\r']) {
        return Err("Steam shortcut paths cannot contain quotes, NUL, or line breaks".into());
    }
    Ok(format!("\"{value}\""))
}

fn same_exe(existing: &str, expected: &str, platform: Platform) -> bool {
    let existing = existing.trim_matches('"');
    let expected = expected.trim_matches('"');
    if platform == Platform::Windows {
        existing
            .replace('\\', "/")
            .eq_ignore_ascii_case(&expected.replace('\\', "/"))
    } else {
        existing == expected
    }
}

fn shortcut_bytes(old: Option<&[u8]>, binary: &Path, platform: Platform) -> Result<(Vec<u8>, u32)> {
    let exe = quoted(binary)?;
    let icon = binary.with_file_name("icon.png");
    let icon = icon
        .to_str()
        .ok_or("Steam icon paths must be valid Unicode")?;
    let start = quoted(
        binary
            .parent()
            .ok_or("Installed executable has no parent directory")?,
    )?;
    let id = crc32fast::hash(format!("{exe}Tempest 2032").as_bytes()) | 0x8000_0000;
    let mut document = if let Some(old) = old {
        Object::parse(old)?
    } else {
        let mut document = Object::new();
        document.set(Field::object("shortcuts", Object::new()));
        document
    };
    let shortcuts = document
        .get_mut("shortcuts")
        .and_then(Field::as_object_mut)
        .ok_or("Binary VDF does not contain a shortcuts object")?;
    let matches: Vec<_> = shortcuts
        .fields
        .iter()
        .enumerate()
        .filter_map(|(index, field)| {
            let object = field.as_object()?;
            (object
                .get("Exe")
                .and_then(Field::as_str)
                .is_some_and(|old| same_exe(old, &exe, platform))
                || object.get("appid").and_then(Field::as_u32) == Some(id))
            .then_some(index)
        })
        .collect();
    if matches.len() > 1 {
        return Err("Multiple existing Tempest 2032 shortcuts match this installation; remove duplicates in Steam first".into());
    }
    let app_id = if let Some(&index) = matches.first() {
        let object = shortcuts.fields[index].as_object_mut().unwrap();
        object.set(Field::string("Exe", &exe));
        if object.get("StartDir").is_none() {
            object.set(Field::string("StartDir", &start));
        }
        if object.get("AppName").is_none() {
            object.set(Field::string("AppName", "Tempest 2032"));
        }
        if object
            .get("icon")
            .is_none_or(|field| field.as_str() == Some(""))
        {
            object.set(Field::string("icon", icon));
        }
        match object.get("appid") {
            Some(field) => field
                .as_u32()
                .ok_or("Existing Tempest 2032 shortcut has an invalid appid")?,
            None => {
                object.set(Field::integer("appid", id));
                id
            }
        }
    } else {
        let mut object = Object::new();
        object.set(Field::integer("appid", id));
        for (key, value) in [
            ("AppName", "Tempest 2032"),
            ("Exe", exe.as_str()),
            ("StartDir", start.as_str()),
            ("icon", icon),
            ("ShortcutPath", ""),
            ("LaunchOptions", "--fullscreen"),
            ("DevkitGameID", ""),
            ("FlatpakAppID", ""),
        ] {
            object.set(Field::string(key, value));
        }
        for (key, value) in [
            ("IsHidden", 0),
            ("AllowDesktopConfig", 1),
            ("AllowOverlay", 1),
            ("openvr", 0),
            ("Devkit", 0),
            ("DevkitOverrideAppID", 0),
            ("LastPlayTime", 0),
        ] {
            object.set(Field::integer(key, value));
        }
        object.set(Field::object("tags", Object::new()));
        let next = shortcuts
            .fields
            .iter()
            .filter_map(|f| f.key()?.parse::<u32>().ok())
            .max()
            .map_or(Some(0), |n| n.checked_add(1))
            .ok_or("Steam shortcut index overflow")?;
        shortcuts
            .fields
            .push(Field::object(&next.to_string(), object));
        id
    };
    Ok((document.bytes(), app_id))
}

#[cfg(test)]
#[path = "tests/steam.rs"]
mod tests;
