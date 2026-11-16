use std::{
    borrow::Borrow,
    hash::{Hash, Hasher},
};

use hashbrown::HashTable;
use rowan::{GreenNode, GreenNodeData, GreenToken, GreenTokenData, NodeOrToken, SyntaxKind};
use rustc_hash::FxHasher;

/// Alias to refer to a green child element.
pub type GreenElement = NodeOrToken<GreenNode, GreenToken>;
/// Alias to refer to a reference of a green child element.
pub type GreenElementRef<'a> = NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>;

#[inline]
pub(super) fn borrow_element(item: &GreenElement) -> GreenElementRef<'_> {
    match item {
        NodeOrToken::Node(node) => NodeOrToken::Node(node),
        NodeOrToken::Token(token) => NodeOrToken::Token(token),
    }
}

#[inline]
fn by_address(item: &GreenElement) -> *const () {
    match item {
        NodeOrToken::Node(node) => node.borrow() as *const GreenNodeData as *const (),
        NodeOrToken::Token(token) => token.borrow() as *const GreenTokenData as *const (),
    }
}
#[inline]
fn by_address_raw(item: GreenElementRef<'_>) -> *const () {
    match item {
        NodeOrToken::Node(node) => node.borrow() as *const GreenNodeData as *const (),
        NodeOrToken::Token(token) => token.borrow() as *const GreenTokenData as *const (),
    }
}

fn hash_node(
    kind: SyntaxKind,
    children: impl Iterator<Item = GreenElement>,
) -> (u64, Vec<GreenElement>) {
    let mut hasher = FxHasher::default();
    let mut resultant = Vec::new();
    kind.hash(&mut hasher);
    for child in children {
        by_address_raw(borrow_element(&child)).hash(&mut hasher);
        resultant.push(child);
    }
    (hasher.finish(), resultant)
}
fn hash_token(kind: SyntaxKind, text: &str) -> u64 {
    let mut hasher = FxHasher::default();
    kind.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}
#[inline]
fn hash_existing_node(node: &GreenNode) -> u64 {
    let mut hasher = FxHasher::default();
    node.kind().hash(&mut hasher);
    for child in node.children() {
        by_address_raw(child).hash(&mut hasher);
    }
    hasher.finish()
}
#[inline]
fn hash_existing_token(token: &GreenToken) -> u64 {
    hash_token(token.kind(), token.text())
}

/// [`GreenNode`] and [`GreenToken`] interner for bottom-up construction.
/// Similar to [`rowan::NodeCache`], but with exposed methods.
///
/// Used within [`ParseStateData`](crate::parse::ParseStateData).
#[derive(Debug, Clone, Default)]
pub struct GreenInterner {
    // These are qualified as pub(super) so that the testing module can use the internals for a has collision test.
    pub(super) nodes: HashTable<GreenNode>,
    pub(super) tokens: HashTable<GreenToken>,
}
impl GreenInterner {
    /// Creates a new interner.
    pub fn new() -> Self {
        GreenInterner {
            nodes: HashTable::new(),
            tokens: HashTable::new(),
        }
    }
    /// Interns a node.
    ///
    /// # Caller Precondition
    ///
    /// This function relies on all children being interned and deduplicated for it to also deduplicate the parent.
    /// Therefore, callers should ensure there are no externally-constructed children to in the children list.
    pub fn node(
        &mut self,
        kind: SyntaxKind,
        collection: impl IntoIterator<Item = GreenElement>,
    ) -> GreenNode {
        let (hash, children) = hash_node(kind, collection.into_iter());
        self.nodes
            .entry(
                hash,
                |existing| {
                    existing.kind() == kind
                        && existing
                            .children()
                            .map(by_address_raw)
                            .eq(children.iter().map(by_address))
                },
                hash_existing_node,
            )
            .or_insert_with(|| GreenNode::new(kind, children))
            .get()
            .clone()
    }
    /// Interns a token.
    pub fn token(&mut self, kind: SyntaxKind, text: &str) -> GreenToken {
        self.tokens
            .entry(
                hash_token(kind, text),
                |existing| existing.kind() == kind && existing.text() == text,
                hash_existing_token,
            )
            .or_insert_with(|| GreenToken::new(kind, text))
            .get()
            .clone()
    }
}
