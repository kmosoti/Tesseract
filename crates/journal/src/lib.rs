//! Single-writer, fsynced, hash-chained JSONL with explicit torn-tail recovery.
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};
use tesseract_contracts::{Error, Experience, ObservationReceipt, Result, SCHEMA, digest};
use tesseract_kernel::{CognitiveKernel, Event, Kernel, Kind, ValidatedRevision};

const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
enum Payload {
    Opened { kind: Kind },
    Transition(Box<Event>),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Envelope {
    schema: u32,
    sequence: usize,
    previous: String,
    payload: Payload,
    hash: String,
}
pub struct Journal {
    file: File,
    sequence: usize,
    head: String,
}
impl Journal {
    fn append(&mut self, payload: Payload) -> Result<()> {
        self.append_batch(vec![payload])
    }
    fn append_batch(&mut self, payloads: Vec<Payload>) -> Result<()> {
        let mut sequence = self.sequence;
        let mut head = self.head.clone();
        let mut bytes = Vec::new();
        for payload in payloads {
            let hash = digest(&(SCHEMA, sequence, &head, &payload))?;
            let row = Envelope {
                schema: SCHEMA,
                sequence,
                previous: head,
                payload,
                hash: hash.clone(),
            };
            serde_json::to_writer(&mut bytes, &row)?;
            bytes.push(b'\n');
            sequence += 1;
            head = hash;
        }
        let original = self.file.metadata()?.len();
        if original + bytes.len() as u64 > MAX_JOURNAL_BYTES {
            return Err(Error::Budget("journal exceeds 64 MiB".into()));
        }
        self.file.seek(SeekFrom::End(0))?;
        if let Err(error) = self
            .file
            .write_all(&bytes)
            .and_then(|()| self.file.sync_all())
        {
            // Restore the prior transaction if the process is still alive.
            self.file.set_len(original)?;
            self.file.sync_all()?;
            return Err(error.into());
        }
        self.sequence = sequence;
        self.head = head;
        Ok(())
    }
    fn open(path: &Path, create: bool, kind: Kind) -> Result<(Self, Kernel)> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(create)
            .open(path)?;
        file.try_lock()
            .map_err(|e| Error::Invalid(format!("journal already has a writer: {e}")))?;
        if file.metadata()?.len() > MAX_JOURNAL_BYTES {
            return Err(Error::Budget("journal exceeds 64 MiB".into()));
        }
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let committed = bytes.iter().rposition(|b| *b == b'\n').map_or(0, |p| p + 1);
        if committed < bytes.len() {
            // Only incomplete final lines may be discarded. Complete corrupt
            // records and missing headers fail closed. The unkeyed chain detects
            // corruption; a trusted external digest is needed against rewriting.
            file.set_len(committed as u64)?;
            file.sync_all()?;
            bytes.truncate(committed);
        }
        let mut sequence = 0;
        let mut head = digest(&"tesseract-journal-v1")?;
        let mut kernel = None;
        for line in bytes.split(|b| *b == b'\n').filter(|b| !b.is_empty()) {
            let envelope: Envelope =
                serde_json::from_slice(line).map_err(|e| Error::Corrupt(e.to_string()))?;
            if envelope.schema != SCHEMA
                || envelope.sequence != sequence
                || envelope.previous != head
                || envelope.hash != digest(&(SCHEMA, sequence, &head, &envelope.payload))?
            {
                return Err(Error::Corrupt("journal hash chain mismatch".into()));
            }
            match envelope.payload {
                Payload::Opened { kind } if sequence == 0 => {
                    kernel = Some(Kernel::new(kind)?);
                }
                Payload::Transition(event) => {
                    kernel
                        .as_mut()
                        .ok_or_else(|| Error::Corrupt("missing header".into()))?
                        .replay_event(*event)?;
                }
                _ => return Err(Error::Corrupt("duplicate or misplaced header".into())),
            }
            sequence += 1;
            head = envelope.hash;
        }
        let mut journal = Self {
            file,
            sequence,
            head,
        };
        let kernel = match kernel {
            Some(k) => k,
            None if create => {
                journal.append(Payload::Opened { kind })?;
                let parent = path
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .unwrap_or_else(|| Path::new("."));
                File::open(parent)?.sync_all()?;
                Kernel::new(kind)?
            }
            None => return Err(Error::Corrupt("missing complete journal header".into())),
        };
        Ok((journal, kernel))
    }
}

pub struct Session {
    kernel: Kernel,
    journal: Journal,
}
impl Session {
    pub fn create(path: impl AsRef<Path>, kind: Kind) -> Result<Self> {
        let (journal, kernel) = Journal::open(path.as_ref(), true, kind)?;
        Ok(Self { kernel, journal })
    }
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let (journal, kernel) = Journal::open(path.as_ref(), false, Kind::Bayesian)?;
        Ok(Self { kernel, journal })
    }
    pub fn store(path: impl AsRef<Path>, kernel: &Kernel) -> Result<Self> {
        let mut session = Self::create(path, kernel.kind())?;
        let payloads = kernel
            .events()
            .iter()
            .cloned()
            .map(|e| Payload::Transition(Box::new(e)))
            .collect();
        session.journal.append_batch(payloads)?;
        session.kernel = kernel.clone();
        Ok(session)
    }
    pub fn kernel(&self) -> &Kernel {
        &self.kernel
    }
    pub fn head(&self) -> &str {
        &self.journal.head
    }
    fn install(&mut self, next: Kernel) -> Result<()> {
        let event = next
            .events()
            .last()
            .ok_or_else(|| Error::Invalid("missing transition".into()))?
            .clone();
        self.journal.append(Payload::Transition(Box::new(event)))?;
        self.kernel = next;
        Ok(())
    }
    pub fn observe(&mut self, e: Experience) -> Result<ObservationReceipt> {
        let mut next = self.kernel.clone();
        let receipt = next.observe(e)?;
        self.install(next)?;
        Ok(receipt)
    }
    pub fn commit(&mut self, proof: ValidatedRevision) -> Result<u64> {
        let mut next = self.kernel.clone();
        let id = next.commit(proof)?;
        self.install(next)?;
        Ok(id)
    }
    pub fn rollback(&mut self, target: u64) -> Result<()> {
        let mut next = self.kernel.clone();
        next.rollback(target)?;
        self.install(next)
    }
    pub fn checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        let snapshot = Checkpoint {
            schema: SCHEMA,
            journal_head: self.head().to_owned(),
            state_hash: digest(&serde_json::to_value(&self.kernel)?)?,
            kernel: serde_json::to_value(&self.kernel)?,
        };
        let parent = path
            .as_ref()
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        serde_json::to_writer(file.as_file_mut(), &snapshot)?;
        file.as_file_mut().sync_all()?;
        file.persist_noclobber(path.as_ref())
            .map_err(|e| Error::Io(e.error))?;
        File::open(parent)?.sync_all()?;
        Ok(())
    }
    /// Checkpoints are verified against replay, never trusted as an alternate truth.
    pub fn verify_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::open(path)?;
        if file.metadata()?.len() > MAX_JOURNAL_BYTES {
            return Err(Error::Budget("checkpoint exceeds 64 MiB".into()));
        }
        let snapshot: Checkpoint = serde_json::from_reader(file)?;
        if snapshot.schema != SCHEMA
            || snapshot.journal_head != self.head()
            || snapshot.state_hash != digest(&serde_json::to_value(&self.kernel)?)?
            || digest(&snapshot.kernel)? != snapshot.state_hash
        {
            return Err(Error::Corrupt(
                "checkpoint does not match verified replay".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct Checkpoint {
    schema: u32,
    journal_head: String,
    state_hash: String,
    kernel: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tesseract_contracts::Budget;
    use tesseract_kernel::ValidationPolicy;
    #[test]
    fn durable_commit_restart_checkpoint_and_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let data = tesseract_workshop::dataset(2, 0.0).unwrap();
        let mut kernel = Kernel::new(Kind::Bayesian).unwrap();
        for e in data.training {
            kernel.observe(e).unwrap();
        }
        let c = kernel.develop(Budget::default(), 0).unwrap();
        let p = kernel
            .validate(c, &data.validation, ValidationPolicy::default())
            .unwrap();
        kernel.commit(p).unwrap();
        let expected = kernel.predict(&data.evaluation[0].query).unwrap();
        let session = Session::store(&path, &kernel).unwrap();
        let snapshot = dir.path().join("snapshot.json");
        session.checkpoint(&snapshot).unwrap();
        assert!(Session::open(&path).is_err());
        drop(session);
        let mut restored = Session::open(&path).unwrap();
        restored.verify_checkpoint(snapshot).unwrap();
        assert_eq!(
            restored
                .kernel()
                .predict(&data.evaluation[0].query)
                .unwrap(),
            expected
        );
        assert!(!restored.kernel().processes().is_empty());
        restored.rollback(0).unwrap();
        assert_eq!(restored.kernel().revision(), 0);
        drop(restored);
        assert_eq!(Session::open(path).unwrap().kernel().revision(), 0);
    }
    #[test]
    fn recover_only_torn_tail_and_reject_complete_corruption() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let mut session = Session::create(&path, Kind::Bayesian).unwrap();
        session
            .observe(
                tesseract_workshop::dataset(1, 0.0)
                    .unwrap()
                    .training
                    .remove(0),
            )
            .unwrap();
        let before = session.kernel().state_digest().unwrap();
        drop(session);
        OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"{\"partial\":")
            .unwrap();
        let restored = Session::open(&path).unwrap();
        assert_eq!(restored.kernel().state_digest().unwrap(), before);
        drop(restored);
        let mut bytes = std::fs::read(&path).unwrap();
        let position = bytes.iter().position(|b| *b == b'1').unwrap();
        bytes[position] = b'9';
        std::fs::write(&path, bytes).unwrap();
        assert!(Session::open(path).is_err());
    }
}
