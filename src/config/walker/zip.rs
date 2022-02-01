use std::cell::RefCell;

use std::collections::HashSet;
use std::io::{Cursor, Error, Read, Seek};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use zip::read::ZipFile;
use zip::ZipArchive;

use crate::config::walker::ConfigSource;

impl<R: Read + Seek> ConfigSource for ZipArchive<R> {
    type DirIter = impl Iterator<Item = PathBuf>;
    type FileIter = impl Iterator<Item = PathBuf>;
    type FileRead = Cursor<Vec<u8>>;

    fn list_dirs(&mut self, path: &Path) -> Self::DirIter {
        let mut seen = HashSet::new();

        for idx in 0..self.len() {
            if let Ok(f) = self.by_index(idx) {
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

        seen.into_iter()
    }

    fn list_all_files(&mut self, path: &Path) -> Self::FileIter {
        let mut seen = vec![];

        for idx in 0..self.len() {
            if let Ok(f) = self.by_index(idx) {
                let name = f.mangled_name();
                if name.starts_with(path) && f.is_file() {
                    seen.push(name);
                }
            }
        }

        seen.into_iter()
    }

    fn get_file(&mut self, path: &Path) -> Option<Self::FileRead> {
        self.by_name(
            path.to_str()
                .expect("Fatal error: non-utf8 config filepath"),
        )
        .ok()
        .map(|mut f| {
            let mut buf = Vec::new();
            buf.reserve_exact(f.size() as usize);
            f.read_exact(buf.as_mut_slice())
                .expect("Fatal error: corrupt zip file");
            Cursor::new(buf)
        })
    }
}
