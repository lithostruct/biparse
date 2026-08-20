
use std::{borrow::Borrow, collections::HashMap, hash::{Hash, Hasher}};

use hashbrown::HashTable;
use rowan::{
    GreenNode, GreenNodeData, GreenToken, GreenTokenData, NodeOrToken, SyntaxKind,
};
use rustc_hash::FxHasher;

pub type GreenItem = NodeOrToken<GreenNode, GreenToken>;
pub type GreenItemRef<'a> = NodeOrToken<&'a GreenNodeData, &'a GreenTokenData>;

#[inline]
fn borrow(item: &GreenItem) -> GreenItemRef<'_> {
    match item {
        NodeOrToken::Node(node) => NodeOrToken::Node(node),
        NodeOrToken::Token(token) => NodeOrToken::Token(token),
    }
}

#[inline]
fn by_address(item: &GreenItem) -> *const () {
    match item {
        NodeOrToken::Node(node)
            => node.borrow() as *const GreenNodeData as *const (),
        NodeOrToken::Token(token)
            => token.borrow() as *const GreenTokenData as *const (),
    }
}
#[inline]
fn by_address_raw(item: GreenItemRef<'_>) -> *const () {
    match item {
        NodeOrToken::Node(node)
            => node.borrow() as *const GreenNodeData as *const (),
        NodeOrToken::Token(token)
            => token.borrow() as *const GreenTokenData as *const (),
    }
}

fn hash_node<'a>(kind: SyntaxKind, children: impl Iterator<Item = GreenItemRef<'a>>) -> u64 {
    let mut hasher = FxHasher::default();
    kind.hash(&mut hasher);
    for child in children {
        by_address_raw(child).hash(&mut hasher);
    }
    hasher.finish()
}
fn hash_token(kind: SyntaxKind, text: &str) -> u64 {
    let mut hasher = FxHasher::default();
    kind.hash(&mut hasher);
    text.hash(&mut hasher);
    hasher.finish()
}
#[inline]
fn hash_existing_node(node: &GreenNode) -> u64 {
    hash_node(node.kind(), node.children())
}
#[inline]
fn hash_existing_token(token: &GreenToken) -> u64 {
    hash_token(token.kind(), token.text())
}

pub(crate) struct GreenInterner {
    nodes: HashTable<GreenNode>,
    tokens: HashTable<GreenToken>,
}
impl GreenInterner {
    /// Interns a node.
    /// 
    /// This function relies on all children being interned and deduplicated for it to also deduplicate the parent.
    /// Therefore, it is not sound to have any externally-constructed children to in the children list.
    pub(crate) fn node(&mut self, kind: SyntaxKind, children: Vec<GreenItem>) -> GreenNode {
        self.nodes.entry(
            hash_node(kind, children.iter().map(borrow)), 
            |existing| {
                existing.kind() == kind && existing.children().map(by_address_raw).eq(children.iter().map(by_address))
            }, hash_existing_node
        ).or_insert_with(|| GreenNode::new(kind, children)).get().clone()
    }
    /// Interns a token.
    pub(crate) fn token(&mut self, kind: SyntaxKind, text: &str) -> GreenToken {
        self.tokens.entry(
            hash_token(kind, text), 
            |existing| {
                existing.kind() == kind && existing.text() == text
            }, hash_existing_token
        ).or_insert_with(|| GreenToken::new(kind, text)).get().clone()
    }
}