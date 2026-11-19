use std::{
    fs,
    io::{self, Write},
    path::Path,
};

pub fn load(path: &Path) -> io::Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}
pub fn save(path: &Path, content: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    let mut file = fs::File::create(&temporary)?;
    file.write_all(content.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> std::path::PathBuf {
        let id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("offwork-test-{}-{id}", std::process::id()))
    }
    #[test]
    fn settings_survive_write_read_and_replacement() {
        let directory = fixture();
        let path = directory.join("settings.json");
        save(&path, "{\"paused\":true}").unwrap();
        assert_eq!(load(&path).unwrap().as_deref(), Some("{\"paused\":true}"));
        save(&path, "{\"paused\":false}").unwrap();
        assert_eq!(load(&path).unwrap().as_deref(), Some("{\"paused\":false}"));
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
    }
    #[test]
    fn missing_settings_return_none_but_read_errors_propagate() {
        let directory = fixture();
        assert!(load(&directory.join("missing.json")).unwrap().is_none());
        fs::create_dir_all(&directory).unwrap();
        assert!(load(&directory).is_err());
        fs::remove_dir(&directory).unwrap();
    }
    #[test]
    fn invalid_destination_reports_write_failure() {
        let directory = fixture();
        fs::create_dir_all(&directory).unwrap();
        let blocker = directory.join("file");
        fs::write(&blocker, "blocker").unwrap();
        assert!(save(&blocker.join("settings.json"), "{}").is_err());
        fs::remove_file(&blocker).unwrap();
        fs::remove_dir(&directory).unwrap();
    }
}
