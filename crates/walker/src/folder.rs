use std::fmt::Formatter;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::ReadSeek;
use crate::ConfigSourceTrait;

pub struct FolderSource(PathBuf);

impl FolderSource {
    pub fn new(path: &Path) -> Option<Self> {
        if path.is_dir() {
            Some(FolderSource(path.to_path_buf()))
        } else {
            None
        }
    }
}

impl std::fmt::Display for FolderSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.0
                .to_str()
                .unwrap_or("<invalid unicode in mod folder name>")
        )
    }
}

impl ConfigSourceTrait for FolderSource {
    fn filesystem_root(&mut self) -> Option<PathBuf> {
        Some(self.0.clone())
    }

    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let path = path.to_path_buf();
        let start = self.0.clone();
        Box::new(
            WalkDir::new(self.0.join(path))
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .map(move |e| e.path().strip_prefix(&start).unwrap().to_path_buf())
                .filter(|p| p.is_dir()),
        )
    }

    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let path = path.to_path_buf();
        let start = self.0.clone();
        Box::new(
            WalkDir::new(self.0.join(path))
                .min_depth(1)
                .into_iter()
                .filter_map(Result::ok)
                .map(move |e| e.path().strip_prefix(&start).unwrap().to_path_buf())
                .filter(|p| !p.is_dir()),
        )
    }

    fn get_file(&mut self, path: &Path) -> Option<Box<dyn ReadSeek>> {
        let file = File::open(self.0.join(path)).ok()?;
        // hey rust stdlib why the FUCK do we have to check this explicitly
        if !file.metadata().ok()?.is_file() {
            return None;
        }
        Some(Box::new(BufReader::new(file)))
    }
}
