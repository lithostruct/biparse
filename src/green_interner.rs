
use rowan::{NodeOrToken, GreenNode, GreenToken};
use hashbrown::HashTable;
use rustc_hash::FxHasher;

pub(crate) struct GreenInterner {
    nodes: HashTable<GreenNode>,
    tokens: HashTable<GreenToken>,
}