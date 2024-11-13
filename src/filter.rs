use std::{
    collections::HashSet,
    hash::{DefaultHasher, Hasher as _},
};

use crate::{message::MessageSource, Message};

/// Filter trait for implementing specific filter types
pub trait Filter: std::fmt::Debug + Send + Sync {
    fn filter(&self, message: &Message) -> bool;
}

#[derive(Default, Debug)]
pub enum FilterOp {
    /// Match any
    #[default]
    Any,
    /// Match all
    All,
    /// Match none
    Negative,
}

/// Message Source filter
#[derive(Debug, Default)]
pub struct SourceFilter {
    op: FilterOp,
    hashes: HashSet<u64>,
}

impl SourceFilter {
    /// Hash a MessageSource, and add it to the filter set
    pub fn add<S: MessageSource>(mut self, source: S) -> Self {
        let mut hasher = DefaultHasher::new();
        source.hash(&mut hasher);
        let hash = hasher.finish();
        self.hashes.insert(hash);
        self
    }
}

impl Filter for SourceFilter {
    fn filter(&self, message: &Message) -> bool {
        if let Some(hash) = message.source_hash() {
            match self.op {
                FilterOp::Any => self.hashes.contains(&hash),
                FilterOp::All => todo!(),
                FilterOp::Negative => todo!(),
            }
        } else {
            false
        }
    }
}
