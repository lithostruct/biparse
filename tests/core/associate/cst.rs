use super::{SExp, Lang};
use biparse::green::GreenInterner;
use expect_test::expect;
use rowan::SyntaxKind;

#[test]
fn single_token() {
    let sexp = SExp::Symbol(String::from("lorem ipsum"));
    let red = sexp.to_cst(&mut GreenInterner::new(), &mut None)
        .canonize::<Lang>(SyntaxKind(99));
    expect![[r#"
        Node(99)@0..11
          Token@0..11 "lorem ipsum"

    "#]].assert_debug_eq(red.tree());
}

#[test]
fn nested() {
    let sexp = SExp::List(SyntaxKind(4), vec![
        SExp::List(SyntaxKind(1), vec![SExp::Symbol(String::from("a"))]),
        SExp::List(SyntaxKind(2), vec![
            SExp::Symbol(String::from("b")),
            SExp::Symbol(String::from("c")),
        ]),
        SExp::List(SyntaxKind(3), vec![
            SExp::Symbol(String::from("d")),
            SExp::Symbol(String::from("e")),
            SExp::Symbol(String::from("f")),
        ]),
    ]);
    let red = sexp.to_cst(&mut GreenInterner::new(), &mut Some(1))
        .canonize::<Lang>(SyntaxKind(99));

    expect![[r#"
        Node(99)@0..6
          Node(4)@0..6
            Node(1)@0..1
              Token@0..1 "a"
            Node(2)@1..3
              Token@1..2 "b"
              Token@2..3 "c"
            Node(3)@3..6
              Token@3..4 "d"
              Token@4..5 "e"
              Token@5..6 "f"

    "#]].assert_debug_eq(red.tree());

    expect![[r#"
        {
            1: CstPath(
                0,
                [
                    0,
                    0,
                ],
            ),
            2: CstPath(
                0,
                [
                    0,
                    1,
                ],
            ),
            3: CstPath(
                0,
                [
                    0,
                    2,
                ],
            ),
            4: CstPath(
                0,
                [
                    0,
                ],
            ),
        }
    "#]].assert_debug_eq(&im::OrdMap::from(red.associations().clone()));

    // Each association should point to the node with equivalent SyntaxKind value.
    for i in 1..=red.associations().len() {
        assert!(red.search(&i).is_some_and(|inner| matches!(inner.kind(), Lang::Node(t) if i == (t as usize))))
    }
}