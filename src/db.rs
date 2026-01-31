use crate::error::CliError;
use crate::model::{default_db, Db};
use crate::stable_json::stable_to_string_pretty;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn resolve_db_path(cli_db_path: Option<&str>) -> Result<String, CliError> {
    if let Some(p) = cli_db_path.map(|s| s.trim()).filter(|s| !s.is_empty()) {
        return Ok(p.to_string());
    }

    if let Ok(p) = std::env::var("HABITCLI_DB_PATH") {
        let p = p.trim().to_string();
        if !p.is_empty() {
            return Ok(p);
        }
    }

    let base = std::env::var("XDG_DATA_HOME")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok());

    let base = match (base, home) {
        (Some(b), _) => b,
        (None, Some(h)) => Path::new(&h)
            .join(".local")
            .join("share")
            .to_string_lossy()
            .to_string(),
        (None, None) => return Err(CliError::io("DB IO error")),
    };

    Ok(Path::new(&base)
        .join("habit-cli")
        .join("db.json")
        .to_string_lossy()
        .to_string())
}

fn validate_db_shape(db: &Db) -> Result<(), CliError> {
    if db.version != 1 {
        return Err(CliError::io("DB corrupted"));
    }
    if db.meta.next_habit_number < 1
        || db.meta.next_declaration_number < 1
        || db.meta.next_excuse_number < 1
        || db.meta.next_penalty_rule_number < 1
    {
        return Err(CliError::io("DB corrupted"));
    }
    Ok(())
}

pub fn read_db(db_path: &str) -> Result<Db, CliError> {
    match fs::read_to_string(db_path) {
        Ok(txt) => {
            let db: Db = serde_json::from_str(&txt).map_err(|_| CliError::io("DB corrupted"))?;
            validate_db_shape(&db)?;
            Ok(db)
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(default_db())
            } else {
                Err(CliError::io("DB IO error"))
            }
        }
    }
}

fn ensure_parent_dir(db_path: &str) -> Result<(), CliError> {
    let dir = Path::new(db_path)
        .parent()
        .ok_or_else(|| CliError::io("DB IO error"))?;
    fs::create_dir_all(dir).map_err(|_| CliError::io("DB IO error"))?;

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(dir, fs::Permissions::from_mode(0o700));
    }

    Ok(())
}

struct WriteLock {
    path: PathBuf,
}

impl Drop for WriteLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn with_write_lock<R>(
    db_path: &str,
    f: impl FnOnce() -> Result<R, CliError>,
) -> Result<R, CliError> {
    let lock_path = PathBuf::from(format!("{}.lock", db_path));

    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(mut file) => {
            #[cfg(unix)]
            {
                let _ = file.set_permissions(fs::Permissions::from_mode(0o600));
            }
            let _ = file.write_all(b"");
            let _guard = WriteLock { path: lock_path };
            f()
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Err(CliError::io("DB is locked"))
            } else {
                Err(CliError::io("DB IO error"))
            }
        }
    }
}

fn write_db_inner(db_path: &str, db: &Db) -> Result<(), CliError> {
    validate_db_shape(db)?;
    ensure_parent_dir(db_path)?;

    let dir = Path::new(db_path)
        .parent()
        .ok_or_else(|| CliError::io("DB IO error"))?;

    let tmp_path = dir.join(format!(".db.json.tmp.{}", std::process::id()));
    let data = stable_to_string_pretty(db).map_err(|_| CliError::io("DB IO error"))? + "\n";

    {
        let mut f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp_path)
            .map_err(|_| CliError::io("DB IO error"))?;

        #[cfg(unix)]
        {
            let _ = f.set_permissions(fs::Permissions::from_mode(0o600));
        }

        f.write_all(data.as_bytes())
            .map_err(|_| CliError::io("DB IO error"))?;
        let _ = f.flush();
    }

    fs::rename(&tmp_path, db_path).map_err(|_| {
        let _ = fs::remove_file(&tmp_path);
        CliError::io("DB IO error")
    })?;

    #[cfg(unix)]
    {
        let _ = fs::set_permissions(db_path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

pub fn update_db<R>(
    db_path: &str,
    mutator: impl FnOnce(&mut Db) -> Result<R, CliError>,
) -> Result<R, CliError> {
    ensure_parent_dir(db_path)?;
    with_write_lock(db_path, || {
        let mut db = read_db(db_path)?;
        let out = mutator(&mut db)?;
        validate_db_shape(&db)?;
        write_db_inner(db_path, &db)?;
        Ok(out)
    })
}
