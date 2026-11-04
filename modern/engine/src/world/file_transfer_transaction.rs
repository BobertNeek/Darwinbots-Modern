//! Keep external transfers recoverable until the simulation cycle succeeds.

use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use crate::EngineError;

#[derive(Default)]
pub(super) struct FileTransferTransaction {
    exports: Vec<(tempfile::TempPath, PathBuf)>,
    inputs: Vec<(PathBuf, PathBuf)>,
    published: usize,
    committed: bool,
}

impl FileTransferTransaction {
    pub fn stage_export(&mut self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        if path.try_exists()? || self.exports.iter().any(|(_, target)| target == path) {
            return Err(std::io::ErrorKind::AlreadyExists.into());
        }
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent)?;
        let mut staged = tempfile::Builder::new()
            .prefix(".db2-output-")
            .suffix(".pending")
            .tempfile_in(parent)?;
        staged.write_all(bytes)?;
        staged.as_file().sync_all()?;
        self.exports
            .push((staged.into_temp_path(), path.to_owned()));
        Ok(())
    }

    pub fn claim_input(&mut self, path: &Path, expected: &[u8]) -> std::io::Result<()> {
        static NEXT_CLAIM: AtomicU64 = AtomicU64::new(0);
        let claim = path.with_file_name(format!(
            ".db2-input-{}-{}.pending",
            std::process::id(),
            NEXT_CLAIM.fetch_add(1, Ordering::Relaxed)
        ));
        // A hard link reserves the recovery name without replacing any file.
        fs::hard_link(path, &claim)?;
        if fs::read(&claim).is_ok_and(|bytes| bytes == expected) {
            if let Err(error) = fs::remove_file(path) {
                let _ = fs::remove_file(&claim);
                return Err(error);
            }
            self.inputs.push((path.to_owned(), claim));
            Ok(())
        } else {
            let _ = fs::remove_file(claim);
            Err(std::io::ErrorKind::InvalidData.into())
        }
    }

    pub fn cancel_export(&mut self, path: &Path) {
        self.exports.retain(|(_, target)| target != path);
    }

    pub fn commit(&mut self) -> Result<(), EngineError> {
        for (staged, target) in &self.exports {
            fs::hard_link(staged, target).map_err(|error| {
                EngineError::Invariant(format!(
                    "cannot publish file transfer {}: {error}",
                    target.display()
                ))
            })?;
            self.published += 1;
        }
        self.committed = true;
        // Failed cleanup leaves a recoverable .pending file, never another
        // inbound .dbo. It must not undo an already committed simulation.
        for (_, claim) in self.inputs.drain(..) {
            let _ = fs::remove_file(claim);
        }
        self.exports.clear();
        Ok(())
    }

    pub fn rollback(&mut self) -> Result<(), EngineError> {
        let mut failures = Vec::new();
        for (staged, target) in self.exports.iter().take(self.published) {
            if !target.exists() {
                continue;
            }
            // Preserve any replacement written by an external participant.
            let unchanged = fs::read(target)
                .ok()
                .zip(fs::read(staged).ok())
                .is_some_and(|(actual, expected)| actual == expected);
            if !unchanged || fs::remove_file(target).is_err() {
                failures.push(format!("published output retained: {}", target.display()));
            }
        }
        self.published = 0;
        self.exports.clear();
        for (original, claim) in self.inputs.drain(..).rev() {
            if fs::hard_link(&claim, &original).is_ok() {
                let _ = fs::remove_file(&claim);
            } else {
                failures.push(format!("input recovery retained: {}", claim.display()));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(EngineError::Invariant(failures.join("; ")))
        }
    }
}

impl Drop for FileTransferTransaction {
    fn drop(&mut self) {
        if !self.committed {
            let _ = self.rollback();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aborted_transfer_restores_input_and_never_exposes_output() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("input.dbo");
        let output = root.path().join("output.dbo");
        fs::write(&input, b"original robot").unwrap();
        {
            let mut transfer = FileTransferTransaction::default();
            transfer.claim_input(&input, b"original robot").unwrap();
            transfer.stage_export(&output, b"departing robot").unwrap();
            assert!(!input.exists());
            assert!(!output.exists());
        }
        assert_eq!(fs::read(&input).unwrap(), b"original robot");
        assert!(!output.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }

    #[test]
    fn publication_collision_keeps_external_file_and_restores_input() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("input.dbo");
        let output = root.path().join("output.dbo");
        fs::write(&input, b"original robot").unwrap();
        let mut transfer = FileTransferTransaction::default();
        transfer.claim_input(&input, b"original robot").unwrap();
        transfer.stage_export(&output, b"departing robot").unwrap();
        fs::write(&output, b"external robot").unwrap();
        assert!(transfer.commit().is_err());
        transfer.rollback().unwrap();
        assert_eq!(fs::read(&input).unwrap(), b"original robot");
        assert_eq!(fs::read(&output).unwrap(), b"external robot");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
    }

    #[test]
    fn successful_transfer_publishes_complete_output_and_consumes_input() {
        let root = tempfile::tempdir().unwrap();
        let input = root.path().join("input.dbo");
        let output = root.path().join("output.dbo");
        fs::write(&input, b"original robot").unwrap();
        let mut transfer = FileTransferTransaction::default();
        transfer.claim_input(&input, b"original robot").unwrap();
        transfer.stage_export(&output, b"departing robot").unwrap();
        transfer.commit().unwrap();
        drop(transfer);
        assert!(!input.exists());
        assert_eq!(fs::read(&output).unwrap(), b"departing robot");
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    }
}
