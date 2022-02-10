use std::io::Read;
use std::path::{Path, PathBuf};
use enum_dispatch::enum_dispatch;

mod embedded;
mod folder;
mod zip;
pub use embedded::EmbeddedSource;
pub use folder::FolderSource;
pub use self::zip::ZipSource;

#[enum_dispatch(ConfigSourceTrait)]
pub enum ConfigSource {
    Embedded(EmbeddedSource),
    Dir(FolderSource),
    Zip(ZipSource)
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
    } else if let Some(zipped) = ZipSource::new(path) {
        Some(zipped.into())
    } else {
        None
    }
}
