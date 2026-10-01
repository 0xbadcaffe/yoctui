//! Target-only account and fakeroot metadata acquisition. Never consult host NSS.
use super::contained;
use rusqlite::{Connection, OpenFlags, limits::Limit};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::Path,
    time::{Duration, Instant},
};

pub(super) fn accounts(root: &Path, relative: &str) -> HashMap<u32, String> {
    let path = root.join(relative);
    let read = || -> Option<String> {
        contained(root, &path).ok()?;
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(path).ok()?;
        if !file.metadata().ok()?.is_file() {
            return None;
        }
        let mut bytes = Vec::new();
        file.take(1024 * 1024 + 1).read_to_end(&mut bytes).ok()?;
        if bytes.len() > 1024 * 1024 {
            return None;
        }
        String::from_utf8(bytes).ok()
    };
    let Some(text) = read() else {
        return HashMap::new();
    };
    let mut names = HashMap::new();
    for line in text.lines() {
        let fields = line.split(':').collect::<Vec<_>>();
        let expected = if relative == "etc/passwd" { 7 } else { 4 };
        if fields.len() != expected {
            continue;
        }
        let name = fields[0];
        let Ok(id) = fields[2].parse::<u32>() else {
            continue;
        };
        if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
            continue;
        }
        // Duplicate IDs with different names are ambiguous, not guessed.
        names
            .entry(id)
            .and_modify(|old: &mut String| {
                if old != name {
                    old.clear();
                }
            })
            .or_insert_with(|| name.into());
    }
    names.retain(|_, name| !name.is_empty());
    names
}

pub(super) struct PseudoMetadata {
    db: Connection,
    deadline: Instant,
}

impl PseudoMetadata {
    pub(super) fn open(root: &Path) -> Option<Self> {
        let parent = root.parent()?;
        let path = parent.join("pseudo/files.db");
        contained(parent, &path).ok()?;
        let metadata = fs::symlink_metadata(&path).ok()?;
        if !metadata.is_file() || metadata.len() > 256 * 1024 * 1024 {
            return None;
        }
        let db = Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_ONLY
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )
        .ok()?;
        db.busy_timeout(Duration::from_millis(50)).ok()?;
        db.set_limit(Limit::SQLITE_LIMIT_LENGTH, 1024 * 1024).ok()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        db.progress_handler(1000, Some(move || Instant::now() >= deadline))
            .ok()?;
        db.execute_batch("PRAGMA query_only=ON; PRAGMA trusted_schema=OFF; BEGIN;")
            .ok()?;
        // Acquire the snapshot/schema once. A busy/missing/corrupt database must
        // not cost one busy timeout for every child in a large directory.
        db.prepare("SELECT mode, uid, gid, dev, ino, deleting FROM files LIMIT 0")
            .ok()?;
        Some(Self { db, deadline })
    }

    #[cfg(unix)]
    pub(super) fn attributes(
        &self,
        path: &Path,
        metadata: &fs::Metadata,
    ) -> Option<(u32, u32, u32)> {
        use std::os::unix::fs::MetadataExt;
        if Instant::now() >= self.deadline {
            return None;
        }
        let mut query = self
            .db
            .prepare_cached(
                "SELECT mode, uid, gid, dev, ino, deleting FROM files WHERE path = ?1 LIMIT 2",
            )
            .ok()?;
        let mut rows = query.query([path.to_str()?]).ok()?;
        let row = rows.next().ok()??;
        let mode = row.get::<_, u32>(0).ok()?;
        let uid = row.get::<_, u32>(1).ok()?;
        let gid = row.get::<_, u32>(2).ok()?;
        let dev = row.get::<_, i64>(3).ok()?;
        let ino = row.get::<_, i64>(4).ok()?;
        if row.get::<_, i64>(5).ok()? != 0
            || dev as u64 != metadata.dev()
            || ino as u64 != metadata.ino()
            || mode & !0o177777 != 0
            || !matches!(
                mode & 0o170000,
                0o010000 | 0o020000 | 0o040000 | 0o060000 | 0o100000 | 0o120000 | 0o140000
            )
            || rows.next().ok()?.is_some()
        {
            return None;
        }
        Some((mode, uid, gid))
    }
}
