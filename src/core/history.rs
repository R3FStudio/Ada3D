//! Transactional history bounded by both snapshot count and retained data.

use crate::document::Document;
use std::collections::VecDeque;

pub const MAX_HISTORY_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_HISTORY_STATES: usize = 32;

#[derive(Clone, Debug)]
struct Snapshot {
    document: Document,
    bytes: usize,
}

impl Snapshot {
    fn new(document: &Document) -> Self {
        Self {
            document: document.clone(),
            bytes: document.estimated_bytes(),
        }
    }

    fn from_owned(document: Document) -> Self {
        let bytes = document.estimated_bytes();
        Self { document, bytes }
    }
}

#[derive(Default, Debug)]
pub struct History {
    undo: VecDeque<Snapshot>,
    redo: VecDeque<Snapshot>,
    pending: Option<Snapshot>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start an edit transaction. A repeated begin replaces the pending one.
    pub fn begin(&mut self, document: &Document) {
        self.pending = None;
        if document.validate().is_err() {
            return;
        }
        let bytes = document.estimated_bytes();
        if self.make_room(bytes) {
            self.pending = Some(Snapshot::new(document));
        }
    }

    /// Commit only a changed document; empty transactions preserve redo.
    pub fn commit(&mut self, document: &Document) -> bool {
        let Some(before) = self.pending.take() else {
            return false;
        };
        if document.validate().is_err() || before.document == *document {
            return false;
        }
        self.redo.clear();
        self.undo.push_back(before);
        self.trim();
        true
    }

    pub fn undo(&mut self, document: &mut Document) -> bool {
        if document.validate().is_err() {
            return false;
        }
        if self.pending.is_some() {
            self.commit(document);
        }
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.make_room(document.estimated_bytes());
        let current = std::mem::replace(document, previous.document);
        self.redo.push_back(Snapshot::from_owned(current));
        self.trim();
        true
    }

    pub fn redo(&mut self, document: &mut Document) -> bool {
        if document.validate().is_err() {
            return false;
        }
        // A real pending change is a new branch and invalidates redo.
        if self.pending.is_some() {
            self.commit(document);
        }
        let Some(next) = self.redo.pop_back() else {
            return false;
        };
        self.make_room(document.estimated_bytes());
        let current = std::mem::replace(document, next.document);
        self.undo.push_back(Snapshot::from_owned(current));
        self.trim();
        true
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.pending = None;
    }

    pub fn state_count(&self) -> usize {
        self.undo.len() + self.redo.len() + usize::from(self.pending.is_some())
    }

    pub fn estimated_memory_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(self.redo.iter())
            .map(|snapshot| snapshot.bytes)
            .sum::<usize>()
            + self.pending.as_ref().map_or(0, |snapshot| snapshot.bytes)
    }

    /// Free old retained snapshots before allocating another canvas clone.
    fn make_room(&mut self, bytes: usize) -> bool {
        if bytes > MAX_HISTORY_BYTES {
            return false;
        }
        while self.state_count() >= MAX_HISTORY_STATES
            || self.estimated_memory_bytes() > MAX_HISTORY_BYTES - bytes
        {
            if self.undo.pop_front().is_none() && self.redo.pop_front().is_none() {
                return false;
            }
        }
        true
    }

    fn trim(&mut self) {
        while self.state_count() > MAX_HISTORY_STATES
            || self.estimated_memory_bytes() > MAX_HISTORY_BYTES
        {
            if self.undo.pop_front().is_none() && self.redo.pop_front().is_none() {
                self.pending = None;
                break;
            }
        }
    }
}
