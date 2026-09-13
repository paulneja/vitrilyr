use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn key(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn root() -> PathBuf {
    directories::BaseDirs::new()
        .map(|d| d.cache_dir().join("lyricglass"))
        .unwrap_or_else(|| std::env::temp_dir().join(format!("lyricglass-{}", std::process::id())))
}

pub async fn read(path: &Path, max: u64) -> Option<Vec<u8>> {
    if tokio::fs::metadata(path).await.ok()?.len() > max {
        return None;
    }
    tokio::fs::read(path).await.ok()
}

pub async fn write(path: &Path, data: &[u8]) -> Result<()> {
    let parent = path.parent().context("Cache path has no parent")?;
    tokio::fs::create_dir_all(parent).await?;
    let temporary = path.with_extension(format!("tmp-{}", std::process::id()));
    tokio::fs::write(&temporary, data).await?;
    tokio::fs::rename(temporary, path).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn cache_creates_directories_and_bounds_reads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/test.json");
        assert!(read(&path, 50).await.is_none());
        write(&path, b"hello").await.unwrap();
        assert_eq!(read(&path, 50).await.unwrap(), b"hello");
        assert!(read(&path, 4).await.is_none());
    }
}
