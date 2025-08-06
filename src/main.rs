use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};
use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use clap::{Arg, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Debug)]
struct Snapshot {
    id: u32,
    files: HashMap<String, String>, // relative path => hash
}

fn main() {
    let matches = Command::new("ftt")
        .about("🕰️ Filesystem Time Travel")
                .subcommand_required(true)
        .subcommand(Command::new("init").arg(arg_path()))
        .subcommand(Command::new("save").arg(arg_path()))
        .subcommand(Command::new("log").arg(arg_path()))
        .subcommand(
           Command::new("rewind")
                .arg(arg_path())
                .arg(Arg::new("back").long("back").required(false))
                .arg(Arg::new("tag").long("tag").required(false)),
        )
        .subcommand(
            Command::new("diff")
                .arg(arg_path())
                .arg(Arg::new("from").long("from").required(false))
                .arg(Arg::new("to").long("to").required(false))
                .arg(Arg::new("from-tag").long("from-tag").required(false))
                .arg(Arg::new("to-tag").long("to-tag").required(false)),

        )
        .subcommand(
            Command::new("tag")
                .arg(arg_path())
                .arg(Arg::new("snapshot").long("snapshot").required(true))
                .arg(Arg::new("label").long("label").required(true)),
        )
        .subcommand(
            Command::new("status")
                .arg(
    Arg::new("path")
        .help("Path to check status for")
        .required(true)
        .value_parser(clap::value_parser!(PathBuf)),
)
        )
        .get_matches();

    match matches.subcommand() {
        Some(("init", m)) => init(m),
        Some(("save", m)) => save(m),
        Some(("log", m)) => show_log(m),
        Some(("rewind", m)) => rewind(m),
        Some(("diff", m)) => diff_snapshots(m),
        Some(("tag", m)) => tag_snapshot(m),
        Some(("status",m)) => {
            let path: &Path = m.get_one::<PathBuf>("path").unwrap();
            let canonical_path = path.canonicalize().expect("Invalid path");
            status(&canonical_path);

        }

        _ => unreachable!(),
    }
}

#[derive(Serialize, Deserialize, Default)]
struct TagMap {
    tags: HashMap<String, u32>, // tag => snapshot ID
}
fn collect_files(path: &Path) -> io::Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut files = vec![];

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| {
            let f = e.file_name().to_string_lossy();
            !f.starts_with(".ftt") // ⛔ ignore .ftt directory
        }) 
    {
        let entry = entry?;
        if entry.file_type().is_file() {
            let rel = entry.path().strip_prefix(path).unwrap().to_path_buf();
            let data = fs::read(entry.path())?;
            files.push((rel, data));
        }
    }

    Ok(files)
}

fn tags_path(path: &Path) -> PathBuf {
    ftt_dir(path).join("tags.json")
}

fn load_tags(path: &Path) -> TagMap {
    let p = tags_path(path);
    if p.exists() {
        let data = fs::read_to_string(p).unwrap_or_default();
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        TagMap::default()
    }
}

fn save_tags(path: &Path, tags: &TagMap) {
    let json = serde_json::to_string_pretty(tags).unwrap();
    fs::write(tags_path(path), json).unwrap();
}
fn tag_snapshot(matches: &ArgMatches) {
    let base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let snapshot_id: u32 = matches.get_one::<String>("snapshot").unwrap().parse().expect("Invalid snapshot ID");
    let label = matches.get_one::<String>("label").unwrap();

    let index = load_index(&base);
    println!("{}", index.len());
    if !index.iter().any(|s| s.id == snapshot_id) {
        eprintln!("❌ No snapshot found with that ID.");
        return;
    }

    let mut tags = load_tags(&base);
    tags.tags.insert(label.to_string(), snapshot_id);
    save_tags(&base, &tags);
    println!("🏷️ Tagged snapshot {} as '{}'", snapshot_id, label);
}

fn diff_snapshots(matches: &ArgMatches) {
    let base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let tags = load_tags(&base);
    let index = load_index(&base);

    let from_id = if let Some(tag) = matches.get_one::<String>("from-tag") {
        *tags.tags.get(tag).expect("❌ from-tag not found.")
    } else if let Some(id_str) = matches.get_one::<String>("from") {
        id_str.parse().expect("Invalid from ID")
    } else {
        panic!("❌ Must provide --from or --from-tag");
    };

    let to_id = if let Some(tag) = matches.get_one::<String>("to-tag") {
        *tags.tags.get(tag).expect("❌ to-tag not found.")
    } else if let Some(id_str) = matches.get_one::<String>("to") {
        id_str.parse().expect("Invalid to ID")
    } else {
        panic!("❌ Must provide --to or --to-tag");
    };

    let from = index
        .iter()
        .find(|s| s.id == from_id)
        .expect("❌ Snapshot not found for --from");

    let to = index
        .iter()
        .find(|s| s.id == to_id)
        .expect("❌ Snapshot not found for --to");

    let from_files = &from.files;
    let to_files = &to.files;

    let mut added = Vec::new();
    let mut deleted = Vec::new();
    let mut modified = Vec::new();

    for (path, hash) in to_files {
        match from_files.get(path) {
            None => added.push(path),
            Some(old_hash) if old_hash != hash => modified.push(path),
            _ => {}
        }
    }

    for path in from_files.keys() {
        if !to_files.contains_key(path) {
            deleted.push(path);
        }
    }

    println!("📊 Diff from ID {} → ID {}", from.id, to.id);
    for path in &added {
        println!("🆕 Added: {}", path);
    }
    for path in &modified {
        println!("✏️ Modified: {}", path);
    }
    for path in &deleted {
        println!("❌ Deleted: {}", path);
    }

    if added.is_empty() && modified.is_empty() && deleted.is_empty() {
        println!("✅ No changes detected.");
    }
}


fn load_ignore_patterns(base: &Path) -> HashSet<String> {
    let mut ignored = HashSet::new();
    let ignore_file = base.join(".fttignore");

    if let Ok(contents) = fs::read_to_string(ignore_file) {
        for line in contents.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                ignored.insert(trimmed.to_string());
            }
        }
    }

    ignored
}

fn arg_path() -> Arg {
    Arg::new("path").required(true)
}

fn ftt_dir(path: &Path) -> PathBuf {
    path.join(".ftt")
}

fn snapshot_dir(path: &Path) -> PathBuf {
    ftt_dir(path).join("snapshots")
}

fn index_path(path: &Path) -> PathBuf {
    ftt_dir(path).join("index.json")
}

fn hash_file(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).ok()?;
    hasher.update(&buffer);
    Some(format!("{:x}", hasher.finalize()))
}

fn relative_path(base: &Path, path: &Path) -> String {
    path.strip_prefix(base).unwrap().to_string_lossy().to_string()
}

fn scan_dir(path: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let ignore = load_ignore_patterns(path);

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let abs_path = entry.path();
        let rel_path = relative_path(path, abs_path);
        if rel_path.starts_with(".ftt") {
            continue; // skip shadow FS
        }

        if should_ignore(&rel_path, &ignore) {
            continue;
        }

        if let Some(hash) = hash_file(abs_path) {
            map.insert(rel_path, hash);
        }
    }

    map
}
fn should_ignore(rel_path: &str, ignored: &HashSet<String>) -> bool {
    for pattern in ignored {
        if pattern.ends_with('/') {
            if rel_path.starts_with(pattern) {
                return true;
            }
        } else if pattern.starts_with("*.") {
            let ext = pattern.trim_start_matches("*.");
            if rel_path.ends_with(ext) {
                return true;
            }
        } else if rel_path.ends_with(pattern) {
            return true;
        }
    }
    false
}


fn load_index(path: &Path) -> Vec<Snapshot> {
    let index_file = index_path(path);
    if !index_file.exists() {
        return vec![];
    }
    let data = fs::read_to_string(index_file).unwrap_or_default();
    serde_json::from_str(&data).unwrap_or_default()
}

fn save_index(path: &Path, index: &[Snapshot]) {
    let data = serde_json::to_string_pretty(index).unwrap();
    fs::write(index_path(path), data).unwrap();
}
fn is_inside_ftt_dir(start: &Path) -> bool {
    let mut current = Some(start);

    while let Some(path) = current {
        if path.join(".ftt").exists() {
            return true;
        }
        current = path.parent();
    }

    false
}

fn init(matches: &ArgMatches) {
    let base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let dir = ftt_dir(&base);
    let snap = snapshot_dir(&base);
    fs::create_dir_all(&snap).unwrap();
    println!("✅ Initialized FTT at {}", dir.display());
}
fn status(path: &Path) -> io::Result<()> {
    let shadow_path = path.join(".ftt");
    let mut changes = vec![];

    let snapshot_path = shadow_path.join("snapshots");

    if !snapshot_path.exists() {
        eprintln!("No snapshots found. Run `ftt snapshot` first.");
        return Ok(());
    }

    let snapshot_files = collect_files(&snapshot_path)?;
    let current_files = collect_files(path)?;

    let snapshot_set: HashSet<_> = snapshot_files.iter().map(|(p, _)| p.clone()).collect();
    let current_set: HashSet<_> = current_files.iter().map(|(p, _)| p.clone()).collect();

    // New files
    for (p, _) in &current_files {
        if !snapshot_set.contains(p) {
            changes.push(format!("🟢 Added: {}", p.display()));
        }
    }

    // Deleted files
    for (p, _) in &snapshot_files {
        if !current_set.contains(p) {
            changes.push(format!("🔴 Deleted: {}", p.display()));
        }
    }

    // Modified files
    for (p, content) in &current_files {
        if let Some((_, old_content)) = snapshot_files.iter().find(|(op, _)| op == p) {
            if content != old_content {
                changes.push(format!("🟡 Modified: {}", p.display()));
            }
        }
    }

    if changes.is_empty() {
        println!("✅ No changes.");
    } else {
        println!("📦 Changes since last snapshot:");
        for c in changes {
            println!("{}", c);
        }
    }

    Ok(())
}

fn save(matches: &ArgMatches) {
    let mut base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    if base.to_str().unwrap() == "." {
        println!("Using current directory as base path.");
        base = env::current_dir().expect("Failed to get current directory");
    }
    let files = scan_dir(&base);
    let blobs = blobs_dir(&base);

    fs::create_dir_all(&blobs).unwrap();

    // Save file contents
    for (rel_path, hash) in &files {
        let blob_path = blob_path(&base, hash);
        if blob_path.exists() {
            continue; // already stored
        }

        let file_path = base.join(rel_path);
        if let Ok(mut src) = File::open(&file_path) {
            let mut dest = File::create(blob_path).unwrap();
            std::io::copy(&mut src, &mut dest).unwrap();
        }
    }

    // Save snapshot
    let mut index = load_index(&base);
    let next_id = index.last().map(|s| s.id + 1).unwrap_or(1);
    let snapshot = Snapshot {
        id: next_id,
        files,
    };

    let json = serde_json::to_string_pretty(&snapshot).unwrap();
    let file_name = format!("{}.json", snapshot.id);
    let path = snapshot_dir(&base).join(&file_name);
    println!("{:?}", path);
    fs::write(&path, &json).unwrap();
    println!("📸 Snapshot saved as ID {}.", snapshot.id);
    index.push(snapshot);
    save_index(&base, &index);
}


fn show_log(matches: &ArgMatches) {
    let base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let index = load_index(&base);

    for snap in index {
        println!("🆔 {}", snap.id);
    }
}
fn blobs_dir(path: &Path) -> PathBuf {
    ftt_dir(path).join("blobs")
}

fn blob_path(base: &Path, hash: &str) -> PathBuf {
    blobs_dir(base).join(hash)
}
fn rewind(matches: &ArgMatches) {
    let base = PathBuf::from(matches.get_one::<String>("path").unwrap());
    let index = load_index(&base);

    let snapshot = if let Some(tag) = matches.get_one::<String>("tag") {
        let tags = load_tags(&base);
        let id = tags.tags.get(tag).expect("❌ Tag not found.");
        index.iter().find(|s| s.id == *id).expect("❌ No snapshot found for tag.")
    } else if let Some(back) = matches.get_one::<String>("back") {
        if let Ok(n) = back.parse::<usize>() {
            if n >= index.len() {
                panic!("❌ You only have {} snapshots", index.len());
            }
            &index[index.len() - 1 - n]
        } else {
            panic!("❌ Invalid --back value. Use number (e.g. 2)");
        }
    } else {
        panic!("❌ Must provide --back or --tag");
    };

    println!("⏪ Rewinding to snapshot ID {}", snapshot.id);

    let current = scan_dir(&base);

    // Restore or overwrite changed/missing files
    for (rel_path, hash) in &snapshot.files {
        let current_hash = current.get(rel_path);
        let file_path = base.join(rel_path);
        if current_hash != Some(hash) {
            println!("↩️ Restoring {}", rel_path);
            let blob = blob_path(&base, hash);
            if let Ok(mut src) = File::open(blob) {
                if let Some(parent) = file_path.parent() {
                    fs::create_dir_all(parent).ok();
                }
                let mut dest = File::create(&file_path).unwrap();
                std::io::copy(&mut src, &mut dest).unwrap();
            }
        }
    }

    // Delete files that exist now but didn't back then
    for rel_path in current.keys() {
        if !snapshot.files.contains_key(rel_path) {
            println!("❌ Deleting {}", rel_path);
            let _ = fs::remove_file(base.join(rel_path));
        }
    }

    // Remove directories that did not exist in the snapshot
    // Collect all directories in the current tree
    let mut current_dirs = Vec::new();
    for entry in WalkDir::new(&base)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
    {
        let rel_path = relative_path(&base, entry.path());
        // Skip .ftt shadow directory
        if rel_path.starts_with(".ftt") {
            continue;
        }
        current_dirs.push(rel_path);
    }
    // Collect all directories referenced by snapshot files
    let mut snapshot_dirs = std::collections::HashSet::new();
    for rel_path in snapshot.files.keys() {
        let mut path = std::path::Path::new(rel_path);
        while let Some(parent) = path.parent() {
            snapshot_dirs.insert(parent.to_string_lossy().to_string());
            path = parent;
        }
    }
    // Remove directories not present in snapshot
    // Remove in reverse order (deepest first)
    current_dirs.sort_by(|a, b| b.len().cmp(&a.len()));
    for dir in current_dirs {
        if !snapshot_dirs.contains(&dir) && !dir.is_empty() {
            let abs_dir = base.join(&dir);
            if abs_dir.exists() {
                println!("🗑️ Removing directory {}", dir);
                let _ = fs::remove_dir_all(abs_dir);
            }
        }
    }

    println!("✅ Rewind complete.");
}


/// Accepts "1d", "5h", "30m"
fn parse_duration(s: &str) -> Option<Duration> {
    let num: i64 = s[..s.len() - 1].parse().ok()?;
    match &s[s.len() - 1..] {
        "d" => Some(Duration::days(num)),
        "h" => Some(Duration::hours(num)),
        "m" => Some(Duration::minutes(num)),
        _ => None,
    }
}

