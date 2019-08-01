use crate::sys::errno_from_ioerror;
use crate::{host, Result};
use std::fs;
use std::os::windows::fs::{symlink_dir, symlink_file};
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
enum SymlinkKind {
    Loop,
    Dangling,
    File,
    Dir,
}

#[derive(Debug)]
pub(crate) struct Symlink {
    source: PathBuf,
    target: PathBuf,
    kind: SymlinkKind,
}

impl Symlink {
    pub(crate) fn new<P: AsRef<Path>>(source: P, target: P) -> Result<Self> {
        let source_path: &Path = source.as_ref();
        let target_path: &Path = target.as_ref();

        if source_path.exists() {
            Err(host::__WASI_EEXIST)
        } else {
            let kind = if target_path.is_file() {
                symlink_file(target_path, source_path).map_err(errno_from_ioerror)?;
                SymlinkKind::File
            } else if target_path.is_dir() {
                symlink_dir(target_path, source_path).map_err(errno_from_ioerror)?;
                SymlinkKind::Dir
            } else if source_path == target_path {
                SymlinkKind::Loop
            } else {
                SymlinkKind::Dangling
            };

            Ok(Self {
                source: source_path.to_owned(),
                target: target_path.to_owned(),
                kind,
            })
        }
    }

    pub(crate) fn target(&self) -> &Path {
        self.target.as_path()
    }

    pub(crate) fn is_dangling(&self) -> bool {
        self.kind == SymlinkKind::Dangling
    }

    pub(crate) fn is_file(&self) -> bool {
        self.kind == SymlinkKind::File
    }

    pub(crate) fn is_dir(&self) -> bool {
        self.kind == SymlinkKind::Dir
    }

    pub(crate) fn symlink_file(&mut self) -> Result<()> {
        symlink_file(&self.target, &self.source).map_err(errno_from_ioerror)?;
        self.kind = SymlinkKind::File;
        Ok(())
    }

    pub(crate) fn unlink_file(&mut self) -> Result<()> {
        fs::remove_file(&self.source).map_err(errno_from_ioerror)?;
        self.kind = SymlinkKind::Dangling;
        Ok(())
    }

    pub(crate) fn symlink_dir(&mut self) -> Result<()> {
        symlink_dir(&self.target, &self.source).map_err(errno_from_ioerror)?;
        self.kind = SymlinkKind::Dir;
        Ok(())
    }

    pub(crate) fn unlink_dir(&mut self) -> Result<()> {
        fs::remove_dir(&self.source).map_err(errno_from_ioerror)?;
        self.kind = SymlinkKind::Dangling;
        Ok(())
    }

    pub(crate) fn read_link(&self) -> Result<PathBuf> {
        match self.kind {
            SymlinkKind::Loop => Ok(self.target.clone()),
            SymlinkKind::Dangling => Ok(self.target.clone()),
            _ => fs::read_link(&self.source).map_err(errno_from_ioerror),
        }
    }

    pub(crate) fn unlink(&self) -> Result<()> {
        match self.kind {
            SymlinkKind::File => fs::remove_file(&self.source).map_err(errno_from_ioerror),
            SymlinkKind::Dir => fs::remove_dir(&self.source).map_err(errno_from_ioerror),
            _ => Ok(()),
        }
    }
}
