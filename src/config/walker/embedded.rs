use std::io;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::slice::Iter;
use include_dir::{Dir, File};

use crate::config::walker::{ConfigSource};

impl<'d> ConfigSource for Dir<'d> {
    type DirIter = impl Iterator<Item = PathBuf>;
    type FileIter = impl Iterator<Item = PathBuf>;
    type FileRead = Cursor<&'d [u8]>;

    fn list_dirs(&mut self, path: &Path) -> Self::DirIter {
        self.dirs().iter().map(|d| d.path().to_owned())
    }

    fn list_all_files(&mut self, path: &Path) -> Self::FileIter {
        self.files().iter().map(|f| f.path().to_owned())
    }

    fn get_file(&mut self, path: &Path) -> Option<Self::FileRead> {
        Dir::get_file(self, path).map(|d| Cursor::new(d.contents()))
    }
}
