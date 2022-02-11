use include_dir::{include_dir, Dir};
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use crate::celeste_mod::walker::ConfigSourceTrait;

const EMBEDDED: Dir = include_dir!("conf");
#[derive(Copy, Clone)]
pub struct EmbeddedSource();

impl ConfigSourceTrait for EmbeddedSource {
    fn filesystem_root(&mut self) -> Option<PathBuf> {
        None
    }

    fn list_dirs(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let (dir, go) = if let Some(dir) = EMBEDDED.get_dir(path) {
            (dir, true)
        } else {
            (EMBEDDED, false)
        };

        Box::new(
            dir.dirs()
                .iter()
                .filter(move |_| go)
                .map(|d| d.path().to_owned()),
        )
    }

    fn list_all_files(&mut self, path: &Path) -> Box<dyn Iterator<Item = PathBuf>> {
        let (dir, go) = if let Some(dir) = EMBEDDED.get_dir(path) {
            (dir, true)
        } else {
            (EMBEDDED, false)
        };

        Box::new(EmbeddedFileIter::new(!go, dir))
    }

    fn get_file(&mut self, path: &Path) -> Option<Box<dyn Read>> {
        EMBEDDED
            .get_file(path)
            .map(|d| -> Box<dyn Read> { Box::new(Cursor::new(d.contents())) })
    }
}

struct EmbeddedFileIter {
    bunk: bool,
    stack: Vec<(Dir<'static>, bool, usize)>,
}

impl EmbeddedFileIter {
    fn new(bunk: bool, start: Dir<'static>) -> Self {
        Self {
            bunk,
            stack: vec![(start, false, 0)],
        }
    }
}

impl Iterator for EmbeddedFileIter {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bunk {
            return None;
        }

        while let Some((cur_dir, doing_dirs, cur_idx)) = self.stack.pop() {
            if doing_dirs {
                if let Some(next_dir) = cur_dir.dirs.get(cur_idx) {
                    self.stack.push((cur_dir, true, cur_idx + 1));
                    self.stack.push((*next_dir, false, 0));
                }
            } else if let Some(file) = cur_dir.files.get(cur_idx) {
                self.stack.push((cur_dir, false, cur_idx + 1));
                return Some(file.path().to_owned());
            } else {
                self.stack.push((cur_dir, true, 0));
            }
        }

        None
    }
}
