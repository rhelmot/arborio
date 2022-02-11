use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::celeste_mod::walker::ConfigSourceTrait;

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

impl ConfigSourceTrait for FolderSource {
    fn filesystem_root(&mut self) -> Option<PathBuf> {
        Some(self.0.clone())
    }

    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let path = path.to_path_buf();
        let start = self.0.clone();
        Box::new(
            WalkDir::new(self.0.join(&path))
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .map(move |e| {
                    e.path()
                        .to_owned()
                        .strip_prefix(&start)
                        .unwrap()
                        .to_path_buf()
                })
                .filter(|p| p.is_dir()),
        )
    }

    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let path = path.to_path_buf();
        let start = self.0.clone();
        Box::new(
            WalkDir::new(self.0.join(&path))
                .min_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
                .map(move |e| {
                    e.path()
                        .to_owned()
                        .strip_prefix(&start)
                        .unwrap()
                        .to_path_buf()
                })
                .filter(|p| !p.is_dir()),
        )
    }

    fn get_file(&mut self, path: &Path) -> Option<Box<dyn Read>> {
        File::open(self.0.join(path))
            .ok()
            .map(|x| -> Box<dyn Read> { Box::new(x) })
    }
}
