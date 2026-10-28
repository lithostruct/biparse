use super::{SExp, Lang};
use biparse::green::GreenInterner;
use expect_test::expect;
use rowan::SyntaxKind;

#[test]
fn single_token() {
    let sexp = SExp::Symbol(String::from("lorem-ipsum"));
    let red = sexp.to_cst(&mut GreenInterner::new(), &mut None)
        .canonize::<Lang>(SyntaxKind(99));
    expect![[r#"
        Node(99)@0..11
          Node(0)@0..11 "lorem-ipsum"

    "#]].assert_debug_eq(red.tree());
}

#[test]
fn nested() {
    let sexp = SExp::List(SyntaxKind(3), vec![
        SExp::List(SyntaxKind(0), vec![SExp::Symbol(String::from("a"))]),
        SExp::List(SyntaxKind(1), vec![
            SExp::Symbol(String::from("b")),
            SExp::Symbol(String::from("c")),
        ]),
        SExp::List(SyntaxKind(2), vec![
            SExp::Symbol(String::from("d")),
            SExp::Symbol(String::from("e")),
            SExp::Symbol(String::from("f")),
        ]),
    ]);
    let red = sexp.to_cst(&mut GreenInterner::new(), &mut Some(0))
        .canonize::<Lang>(SyntaxKind(100));

    expect![[r#"
        Node(100)@0..6
          Node(3)@0..6
            Node(0)@0..1
              Node(0)@0..1 "a"
            Node(1)@1..3
              Node(0)@1..2 "b"
              Node(0)@2..3 "c"
            Node(2)@3..6
              Node(0)@3..4 "d"
              Node(0)@4..5 "e"
              Node(0)@5..6 "f"

    "#]].assert_debug_eq(red.tree());

    expect![[r#"
        {
            0: CstPath(
                0,
                [
                    0,
                    0,
                ],
            ),
            1: CstPath(
                0,
                [
                    0,
                    1,
                ],
            ),
            2: CstPath(
                0,
                [
                    0,
                    2,
                ],
            ),
            3: CstPath(
                0,
                [
                    0,
                ],
            ),
        }
    "#]].assert_debug_eq(&im::OrdMap::from(red.associations().clone()));

    // Each association should point to the node with equivalent SyntaxKind value.
    for i in 0..red.associations().len() {
        assert!(red.search(&i).is_some_and(|inner| matches!(inner.kind(), Lang::Node(t) if i == (t as usize))))
    }
}