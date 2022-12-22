use enum_dispatch::enum_dispatch;
use std::fmt::{Display, Formatter};
use std::io::{BufRead, Seek};
use std::path::{Path, PathBuf};

pub use crate::embedded::EmbeddedSource;
pub use crate::folder::FolderSource;
pub use crate::zip::ZipSource;

mod embedded;
mod folder;
mod zip;

#[enum_dispatch(ConfigSourceTrait)]
pub enum ConfigSource {
    Embedded(EmbeddedSource),
    Dir(FolderSource),
    Zip(ZipSource),
}

impl Display for ConfigSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSource::Embedded(s) => s.fmt(f),
            ConfigSource::Dir(s) => s.fmt(f),
            ConfigSource::Zip(s) => s.fmt(f),
        }
    }
}

pub trait ReadSeek: BufRead + Seek {}

impl<T> ReadSeek for T where T: BufRead + Seek {}

#[enum_dispatch]
pub trait ConfigSourceTrait {
    fn filesystem_root(&mut self) -> Option<PathBuf>;
    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>>;
    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>>;
    fn get_file(&mut self, path: &Path) -> Option<Box<dyn ReadSeek>>;
}

pub fn open_module(path: &Path) -> Option<ConfigSource> {
    FolderSource::new(path)
        .map(FolderSource::into)
        .or_else(|| ZipSource::new(path).map(ZipSource::into))
}
