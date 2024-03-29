use std::collections::HashSet;
use std::fmt::Formatter;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

use crate::ConfigSourceTrait;

use super::ReadSeek;

pub struct ZipSource {
    path: PathBuf,
    archive: ZipArchive<File>,
}

impl ZipSource {
    pub fn new(path: &Path) -> Option<Self> {
        File::open(path)
            .ok()
            .and_then(|f| ZipArchive::new(f).ok())
            .map(|a| ZipSource {
                path: path.to_path_buf(),
                archive: a,
            })
    }
}

impl std::fmt::Display for ZipSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.path
                .to_str()
                .unwrap_or("<invalid unicode in mod zip name>")
        )
    }
}

impl ConfigSourceTrait for ZipSource {
    fn filesystem_root(&mut self) -> Option<PathBuf> {
        Some(self.path.clone())
    }

    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let mut seen = HashSet::new();

        for idx in 0..self.archive.len() {
            if let Ok(f) = self.archive.by_index(idx) {
                if f.is_dir() {
                    let name = f.mangled_name();
                    if let Ok(rest) = name.strip_prefix(path) {
                        if rest.components().count() == 1 {
                            seen.insert(name);
                        }
                    }
                }
            }
        }

        Box::new(seen.into_iter())
    }

    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let mut seen = vec![];

        for idx in 0..self.archive.len() {
            if let Ok(f) = self.archive.by_index(idx) {
                let name = f.mangled_name();
                if name.starts_with(path) && f.is_file() {
                    seen.push(name);
                }
            }
        }

        Box::new(seen.into_iter())
    }

    fn get_file(&mut self, path: &Path) -> Option<Box<dyn ReadSeek>> {
        self.archive
            .by_name(
                path.to_str()
                    .expect("Fatal error: non-utf8 celeste_mod filepath"),
            )
            .ok()
            .map(|mut f| -> Box<dyn ReadSeek> {
                let mut buf = vec![];
                f.read_to_end(&mut buf)
                    .expect("Fatal error: corrupt zip file");
                Box::new(Cursor::new(buf))
            })
    }
}
