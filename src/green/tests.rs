#[cfg(test)]
mod test {
    //! Unit tests regarding `GreenInterner`.
    //!
    //! 1. Identical tokens are deduplicated.
    //! 2. Differing tokens remain separate.
    //! 3. Identical nodes are deduplicated.
    //! 4. Node kind affects identity.
    //! 5. Node children order affects identity.
    //! 6. Nested bottom-up construction deduplicates.
    //! 7. Hash table growth does not invalidate underlying identities.
    //! 8. Colliding hashes still perform correct structural equality.

    use crate::green::green_interner::GreenInterner;
    use rowan::{GreenNodeData, GreenToken, GreenTokenData, NodeOrToken, SyntaxKind};
    use std::borrow::Borrow;

    // Legally distinct `L : Language` implementation.
    const IDENT: SyntaxKind = SyntaxKind(0);
    const NAME: SyntaxKind = SyntaxKind(1);
    const PAIR: SyntaxKind = SyntaxKind(2);
    const EXPR: SyntaxKind = SyntaxKind(10);
    const ROOT: SyntaxKind = SyntaxKind(100);

    /// 1. Identical tokens are deduplicated.
    #[test]
    fn identical_tokens_are_deduplicated() {
        let mut interner = GreenInterner::new();

        let a = interner.token(SyntaxKind(0), "foo");
        let b = interner.token(SyntaxKind(0), "foo");

        assert!(std::ptr::eq::<GreenTokenData>(a.borrow(), b.borrow()));
    }
    /// 2. Differing tokens remain separate.
    #[test]
    fn different_tokens_remain_separate() {
        let mut interner = GreenInterner::new();

        let a = interner.token(SyntaxKind(0), "foo");
        let b = interner.token(SyntaxKind(0), "bar");

        assert!(!std::ptr::eq::<GreenTokenData>(a.borrow(), b.borrow()));
    }

    /// 3. Identical nodes are deduplicated.
    #[test]
    fn identical_nodes_are_deduplicated() {
        let mut interner = GreenInterner::new();

        let foo = interner.token(SyntaxKind(0), "foo");

        let a = interner.node(SyntaxKind(1), vec![NodeOrToken::Token(foo.clone())]);

        let b = interner.node(SyntaxKind(1), vec![NodeOrToken::Token(foo)]);

        assert!(std::ptr::eq::<GreenNodeData>(a.borrow(), b.borrow()));
    }
    /// 4. Node kind affects identity.
    #[test]
    fn different_children_produce_different_nodes() {
        let mut interner = GreenInterner::new();

        let foo = interner.token(IDENT, "foo");
        let bar = interner.token(IDENT, "bar");

        let a = interner.node(NAME, vec![NodeOrToken::Token(foo)]);

        let b = interner.node(NAME, vec![NodeOrToken::Token(bar)]);

        assert!(!std::ptr::eq::<GreenNodeData>(a.borrow(), b.borrow()))
    }
    /// 5. Node children order affects identity.
    #[test]
    fn child_order_matters() {
        let mut interner = GreenInterner::new();

        let a = interner.token(IDENT, "a");
        let b = interner.token(IDENT, "b");

        let a_b = interner.node(
            PAIR,
            vec![NodeOrToken::Token(a.clone()), NodeOrToken::Token(b.clone())],
        );

        let b_a = interner.node(
            PAIR, 
            vec![NodeOrToken::Token(b), NodeOrToken::Token(a)]
        );

        assert!(!std::ptr::eq::<GreenNodeData>(a_b.borrow(), b_a.borrow()))
    }

    /// 6. Nested bottom-up construction deduplicates.
    #[test]
    fn nested_nodes_are_deduplicated() {
        let mut interner = GreenInterner::new();

        let foo = interner.token(IDENT, "foo");

        let name = interner.node(NAME, vec![NodeOrToken::Token(foo.clone())]);
        let expr = interner.node(EXPR, vec![NodeOrToken::Node(name.clone())]);
        let root_a = interner.node(ROOT, vec![NodeOrToken::Node(expr.clone())]);

        let name_again = interner.node(NAME, vec![NodeOrToken::Token(foo)]);
        let expr_again = interner.node(EXPR, vec![NodeOrToken::Node(name_again)]);
        let root_b = interner.node(ROOT, vec![NodeOrToken::Node(expr_again)]);

        let name_in_b = match root_b.children().next().unwrap() {
            NodeOrToken::Node(node) => node.children().next().unwrap().into_node().unwrap(),
            _ => unreachable!(),
        };

        assert!(std::ptr::eq::<GreenNodeData>(
            name.borrow(),
            name_in_b.borrow()
        ));
        assert!(std::ptr::eq::<GreenNodeData>(
            root_a.borrow(),
            root_b.borrow()
        ));
    }

    /// 7. Hash table growth does not invalidate underlying identities.
    #[test]
    fn hash_table_growth_does_not_invalidate() {
        let mut interner = GreenInterner::new();

        let foo = interner.token(IDENT, "foo");

        let name = interner.node(NAME, vec![NodeOrToken::Token(foo.clone())]);

        for i in 0..10_000 {
            let token = interner.token(IDENT, &i.to_string());
            let _ = interner.node(NAME, vec![NodeOrToken::Token(token)]);
        }

        let name_again = interner.node(NAME, vec![NodeOrToken::Token(foo)]);

        assert!(std::ptr::eq::<GreenNodeData>(
            name.borrow(),
            name_again.borrow()
        ));
    }

    /// 8. Colliding hashes still perform correct structural equality.
    impl GreenInterner {
        fn bad_token(&mut self, kind: SyntaxKind, text: &str) -> GreenToken {
            self.tokens
                .entry(
                    0,
                    |existing| existing.kind() == kind && existing.text() == text,
                    |_| 0,
                )
                .or_insert_with(|| GreenToken::new(kind, text))
                .get()
                .clone()
        }
    }
    #[test]
    fn colliding_hashes_perform_structural_equality() {
        let mut interner = GreenInterner::new();

        let a = interner.bad_token(SyntaxKind(0), "foo");
        let b = interner.bad_token(SyntaxKind(0), "foo");

        assert!(std::ptr::eq::<GreenTokenData>(a.borrow(), b.borrow()));

        let a = interner.bad_token(SyntaxKind(0), "foo");
        let b = interner.bad_token(SyntaxKind(0), "bar");

        assert!(!std::ptr::eq::<GreenTokenData>(a.borrow(), b.borrow()));
    }
}
