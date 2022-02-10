use enum_dispatch::enum_dispatch;
use std::io::Read;
use std::path::{Path, PathBuf};

mod embedded;
mod folder;
mod zip;
pub use self::zip::ZipSource;
pub use embedded::EmbeddedSource;
pub use folder::FolderSource;

#[enum_dispatch(ConfigSourceTrait)]
pub enum ConfigSource {
    Embedded(EmbeddedSource),
    Dir(FolderSource),
    Zip(ZipSource),
}

#[enum_dispatch]
pub trait ConfigSourceTrait {
    fn filesystem_root(&mut self) -> Option<PathBuf>;
    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>>;
    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>>;
    fn get_file(&mut self, path: &Path) -> Option<Box<dyn Read>>;
}

pub fn open_module(path: &Path) -> Option<ConfigSource> {
    if let Some(folder) = FolderSource::new(path) {
        Some(folder.into())
    } else {
        ZipSource::new(path).map(|zipped| zipped.into())
    }
}
