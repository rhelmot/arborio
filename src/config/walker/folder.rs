use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

use crate::config::walker::ConfigSource;

pub struct FolderSource(PathBuf);

impl FolderSource {
    pub fn new(path: PathBuf) -> Option<Self> {
        if path.is_dir() {
            Some(FolderSource(path))
        } else {
            None
        }
    }
}

impl ConfigSource for FolderSource {
    type DirIter = impl Iterator<Item = PathBuf>;
    type FileIter = impl Iterator<Item = PathBuf>;
    type FileRead = File;

    // TODO: does walkdir return absolute or relative paths?

    fn list_dirs(&mut self, path: &Path) -> Self::DirIter {
        WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .filter(|p| p.is_dir())
    }

    fn list_all_files(&mut self, path: &Path) -> Self::FileIter {
        WalkDir::new(path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|e| e.path().to_owned())
            .filter(|p| !p.is_dir())
    }

    fn get_file(&mut self, path: &Path) -> Option<Self::FileRead> {
        File::open(self.0.join(path)).ok()
    }
}
