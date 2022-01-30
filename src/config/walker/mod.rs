use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

mod embedded;
mod folder;
mod zip;

pub trait ConfigSource {
    type DirIter: Iterator<Item = PathBuf>;
    type FileIter: Iterator<Item = PathBuf>;
    type FileRead: Read;

    fn list_dirs(&mut self, path: &Path) -> Self::DirIter;
    fn list_all_files(&mut self, path: &Path) -> Self::FileIter;
    fn get_file(&mut self, path: &Path) -> Option<Self::FileRead>;
}
