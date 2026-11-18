# S-expressions

*Let’s parse simple sexps!*

An S-expression consists of an atom or a nested ordered list of symbolic expression children. Let's assume that we only admit alphanumeric identifiers with underscores and dashes as atoms for now.

```rust
##[derive(Debug, Clone, PartialEq, Eq)]
pub enum SExp {
    Atom(String),
    List(Vec<SExp>),
}
```

Written as [W3C-flavoured Extended Backus–Naur form](https://www.w3.org/2001/06/blindfold/grammar) (EBNF), it may look something like this:

```text
sexp ::= atom | '(' sexp* ')'
atom ::= [a_zA-Z0-9_-]+
```

Simple and straightforward! We'll read off of this when writing the parsers later.

## Groundwork


### `Input` Selection

A parser needs a specific input type. Which type should be selected is largely dependent on its treatment of whitespace.

While whitespace is used to separate different items, it is ultimately not considered important to the syntax of S-expressions. Hence, there are a couple of options here regarding which [`Input`] type to select:
- Use [`str`], with explicit matching on optional whitespace and discarding.
- Use [`LexerStream`], which requires defining a [`logos::Logos`] grammar.  
  This can be done when lexing of tokens are globally unambiguous, which is the case here.
- Use [`SkippingStr`](input::skipping_str::SkippingStr), which lets us use familiar [`str`] matching while implicitly skipping whitespace.  
  The [typical footguns](../input_comparison/3_input_skippingstr.md#tradeoffs) with this type don't apply here,
  since no syntactic construct *uses* whitespace.

We'll go through this example with `SkippingStr` first. Later, we'll see how this can be done with `LexerStream` as well.

### CST Kinds

Since we need to think about CST materialization, let's define our own set of syntax kinds.

```rust
##[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
##[repr(u16)]
enum Kind {
    // Terminals.
    Whitespace = 0,
    LeftParen,
    RightParen,
    Atom,

    // Non-terminals
    List,
    Root,
}
```
Our custom kinds should implement[`Into<rowan::SyntaxKind>`], and optionally [`rowan::Language`]. You would typically use [`num_derive`](https://docs.rs/num-derive/latest/num_derive/) for this purpose (or `unsafe` code, for that matter), but since this example is so minimal I'll just pattern match on all the variants:

<details>
<summary><i>Click to expand.</i></summary>

```rust
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
# #[repr(u16)]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     LeftParen,
#     RightParen,
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
#
impl rowan::Language for Kind {
    type Kind = Self;
    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        match raw.0 {
            0 => Kind::Whitespace,
            1 => Kind::LeftParen,
            2 => Kind::RightParen,
            3 => Kind::Atom,
            4 => Kind::List,
            5 => Kind::Root,
            _ => unreachable!(),
        }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(
            match kind {
                Kind::Whitespace => 0,
                Kind::LeftParen => 1,
                Kind::RightParen => 2,
                Kind::Atom => 3,
                Kind::List => 4,
                Kind::Root => 5,
            }
        )
    }
}
impl Into<rowan::SyntaxKind> for Kind {
    fn into(self) -> rowan::SyntaxKind {
        <Kind as rowan::Language>::kind_to_raw(self)
    }
}
```

</details>

You may choose to use raw [`rowan::SyntaxKind`] values. In fact, that's pretty common within the test suite! However, it's easier to mess up the loose coupling between CST syntax kinds and patterns. Isolating this makes writing the parsing logic later clearer.

If you wish to use raw syntax kinds, Biparse provides [`cst::NoLang`], which is the idempotent [`rowan::Language`] implementation for kind [`rowan::SyntaxKind`].

### Limitations on Error Recovery

Biparse's notion of error recovery makes [panic-mode error recovery](https://www.geeksforgeeks.org/compiler-design/error-recovery-strategies-in-compiler-design/) easy, but may be a bad fit for s-expressions. In fact, `)` *isn't* a safe synchronization character because nested lists eg. `"(!error ) resumed )"` may cause false positives for recovery!

We make the simplification that parsing of an S-expression makes no attempt at error recovery. All failures become terminating.

## `SkippingStr` Implementation

### Step by Step

The parser-producing function requires a lifetime on the borrow on the input, and the parse environment passed:

```rust
use biparse::{
    prelude::*,
    input::skipping_str::SkippingStr,
};
# 
# #[derive(Debug, Clone)]
# pub enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }

fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
    -> impl Parser<'parse, SkippingStr<'source>, SExp, E>
where 'source : 'parse {
    # return success().map(|_| todo!());
    todo!()
}
```

Let's examine `atom` first. It is a terminal corresponding to a specific regex pattern. Fortunately, Biparse comes inbuilt with [`regex`] support! `atom` hence corresponds to:

```rust
# use biparse::{
#     prelude::*,
#     input::skipping_str::SkippingStr,
# };
# 
# #[derive(Debug, Clone)]
# pub enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }
#
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
# #[repr(u16)]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     LeftParen,
#     RightParen,
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
# impl rowan::Language for Kind {
#     type Kind = Self;
#     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
#         match raw.0 {
#             0 => Kind::Whitespace,
#             1 => Kind::LeftParen,
#             2 => Kind::RightParen,
#             3 => Kind::Atom,
#             4 => Kind::List,
#             5 => Kind::Root,
#             _ => unreachable!(),
#         }
#     }
#     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
#         rowan::SyntaxKind(
#             match kind {
#                 Kind::Whitespace => 0,
#                 Kind::LeftParen => 1,
#                 Kind::RightParen => 2,
#                 Kind::Atom => 3,
#                 Kind::List => 4,
#                 Kind::Root => 5,
#             }
#         )
#     }
# }
# impl Into<rowan::SyntaxKind> for Kind {
#     fn into(self) -> rowan::SyntaxKind {
#         <Kind as rowan::Language>::kind_to_raw(self)
#     }
# }
# fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
#     -> impl Parser<'parse, SkippingStr<'source>, SExp, E>
# where 'source : 'parse {
    # return success().map(|_| todo!());
let atom = just::<'parse, SkippingStr<'source>, E, _>(
    (Kind::Atom, regex::Regex::new("[a-zA-Z0-9_-]+").unwrap())
);
    # todo!()
# }
```

The `atom` parser currently produces a `&str` payload on success. Keep this in mind.

The root parser which we are returning corresponds to the `sexp` rule, which is recursive. Hence:
```rust
# use biparse::{
#     prelude::*,
#     input::skipping_str::SkippingStr,
# };
# 
# #[derive(Debug, Clone)]
# pub enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }
#
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
# #[repr(u16)]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     LeftParen,
#     RightParen,
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
# impl rowan::Language for Kind {
#     type Kind = Self;
#     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
#         match raw.0 {
#             0 => Kind::Whitespace,
#             1 => Kind::LeftParen,
#             2 => Kind::RightParen,
#             3 => Kind::Atom,
#             4 => Kind::List,
#             5 => Kind::Root,
#             _ => unreachable!(),
#         }
#     }
#     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
#         rowan::SyntaxKind(
#             match kind {
#                 Kind::Whitespace => 0,
#                 Kind::LeftParen => 1,
#                 Kind::RightParen => 2,
#                 Kind::Atom => 3,
#                 Kind::List => 4,
#                 Kind::Root => 5,
#             }
#         )
#     }
# }
# impl Into<rowan::SyntaxKind> for Kind {
#     fn into(self) -> rowan::SyntaxKind {
#         <Kind as rowan::Language>::kind_to_raw(self)
#     }
# }
fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
    -> impl Parser<'parse, SkippingStr<'source>, SExp, E>
where 'source : 'parse {

    let atom = just::<'parse, SkippingStr<'source>, E, _>(
        (Kind::Atom, regex::Regex::new("[a-zA-Z0-9_-]+").unwrap())
    );
    # let _ : Recursive<dyn Parser<SkippingStr<'source>, SExp, E>> =
    Recursive::new(|sexp| {
        # return success().map(|_| todo!());
        todo!()
    })
    # ; return success().map(|_| todo!());
}
```

The base case of a symbolic expression is `atom`. To transform the `&str` payload into `SExp`, we chain `atom` to [`.into_owned()`](ParserIntoOwnedExt::into_owned) (producing a [`String`]),
then into [`.map(SExp::Atom)`](ParserExt::map) to produce our `SExp` value.

For the recursive case, we use the back edge provided in the closure (what we named as `sexp`). Delimiters are chained via [`.then()`](ParserExt::then) and friends, and [`sexp.many()`](ParserExt::many) produces the `Vec<SExp>` that `SExp::List` expects.

Hence:

```rust
# use biparse::{
#     prelude::*,
#     input::skipping_str::SkippingStr,
# };
# 
# #[derive(Debug, Clone)]
# pub enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }
#
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
# #[repr(u16)]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     LeftParen,
#     RightParen,
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
# impl rowan::Language for Kind {
#     type Kind = Self;
#     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
#         match raw.0 {
#             0 => Kind::Whitespace,
#             1 => Kind::LeftParen,
#             2 => Kind::RightParen,
#             3 => Kind::Atom,
#             4 => Kind::List,
#             5 => Kind::Root,
#             _ => unreachable!(),
#         }
#     }
#     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
#         rowan::SyntaxKind(
#             match kind {
#                 Kind::Whitespace => 0,
#                 Kind::LeftParen => 1,
#                 Kind::RightParen => 2,
#                 Kind::Atom => 3,
#                 Kind::List => 4,
#                 Kind::Root => 5,
#             }
#         )
#     }
# }
# impl Into<rowan::SyntaxKind> for Kind {
#     fn into(self) -> rowan::SyntaxKind {
#         <Kind as rowan::Language>::kind_to_raw(self)
#     }
# }
fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
    -> impl Parser<'parse, SkippingStr<'source>, SExp, E>
where 'source : 'parse {

    let atom = just::<'parse, SkippingStr<'source>, E, _>(
        (Kind::Atom, regex::Regex::new("[a-zA-Z0-9_-]+").unwrap())
    );
    Recursive::new(|sexp|
        atom.into_owned().map(SExp::Atom)                 // <--- inserted:
        .or(
            just((Kind::LeftParen, '('))
            .ignore_then(
                sexp.many().map(SExp::List)
            ).then_ignore(just((Kind::RightParen, ')')))  // <--- end of insert
        )
    )
}
```

That's it for the AST component! For the CST component, we have the two distinct structures of atoms and list. All we really need to do is add [`node`](ParserExt::node) or [`named`](ParserExt::named) at appropriate boundaries:

```rust
# use biparse::{
#     prelude::*,
#     input::skipping_str::SkippingStr,
# };
# 
# #[derive(Debug, Clone)]
# pub enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }
#
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
# #[repr(u16)]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     LeftParen,
#     RightParen,
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
# impl rowan::Language for Kind {
#     type Kind = Self;
#     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
#         match raw.0 {
#             0 => Kind::Whitespace,
#             1 => Kind::LeftParen,
#             2 => Kind::RightParen,
#             3 => Kind::Atom,
#             4 => Kind::List,
#             5 => Kind::Root,
#             _ => unreachable!(),
#         }
#     }
#     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
#         rowan::SyntaxKind(
#             match kind {
#                 Kind::Whitespace => 0,
#                 Kind::LeftParen => 1,
#                 Kind::RightParen => 2,
#                 Kind::Atom => 3,
#                 Kind::List => 4,
#                 Kind::Root => 5,
#             }
#         )
#     }
# }
# impl Into<rowan::SyntaxKind> for Kind {
#     fn into(self) -> rowan::SyntaxKind {
#         <Kind as rowan::Language>::kind_to_raw(self)
#     }
# }
fn sexp_parser<'source, 'parse, E: ParseEnvironment>()
    -> impl Parser<'parse, SkippingStr<'source>, SExp, E>
where 'source : 'parse {

    let atom = just::<'parse, SkippingStr<'source>, E, _>(
        (Kind::Atom, regex::Regex::new("[a-zA-Z0-9_-]+").unwrap())
    );
    Recursive::new(|sexp|
        atom.into_owned().map(SExp::Atom)
            .named("atom", Kind::Atom)                    // <--- inserted.
        .or(
            just((Kind::LeftParen, '('))
            .ignore_then(
                sexp.many().map(SExp::List)
            ).then_ignore(just((Kind::RightParen, ')')))
            .named("list", Kind::List)                    // <--- inserted.
        )
    )
}
```

That's it!

### Trying It Out

We can now try out a parse, by constructing the whitespace-skipping input with `Kind::Whitespace` as the syntax kind of skipped trivia:

```rust,ignore
let input = SkippingStr::whitespace("((1) (2 3) (4 5 6))", Kind::Whitespace);
let result = sexp_parser::<environment::Rich>().parse_all(&input);
```

And unwrapping the artifact within (since we know our input `"((1) (2 3) (4 5 6))"` to be well-formed):

```rust,ignore
let artifact = result.expect("parse success");
```

The AST component is the `SExp` we constructed:

<details>
<summary><i>Click to expand.</i></summary>

```rust,ignore
assert_eq!(*artifact.ast_only(),
    SExp::List(vec![
        SExp::List(vec![
            SExp::Atom("1".to_string()),
        ]),
        SExp::List(vec![
            SExp::Atom("2".to_string()),
            SExp::Atom("3".to_string()),
        ]),
        SExp::List(vec![
            SExp::Atom("4".to_string()),
            SExp::Atom("5".to_string()),
            SExp::Atom("6".to_string()),
        ]),
    ])
);
```

</details>

And the CST component corresponds to the lossless tree. To enable `Debug`, we canonize the green tree with the `Kind::Root` we have saved up until now:

<details>
<summary><i>Click to expand.</i></summary>

```rust,ignore
use expect_test::expect;

let red_tree = artifact.cst().clone().canonize::<Kind>(Kind::Root);
expect![[r#"
    Root@0..19
      List@0..19
        LeftParen@0..1 "("
        List@1..5
          LeftParen@1..2 "("
          Atom@2..3
            Atom@2..3 "1"
          RightParen@3..4 ")"
          Whitespace@4..5 " "
        List@5..11
          LeftParen@5..6 "("
          Atom@6..8
            Atom@6..7 "2"
            Whitespace@7..8 " "
          Atom@8..9
            Atom@8..9 "3"
          RightParen@9..10 ")"
          Whitespace@10..11 " "
        List@11..18
          LeftParen@11..12 "("
          Atom@12..14
            Atom@12..13 "4"
            Whitespace@13..14 " "
          Atom@14..16
            Atom@14..15 "5"
            Whitespace@15..16 " "
          Atom@16..17
            Atom@16..17 "6"
          RightParen@17..18 ")"
        RightParen@18..19 ")"

"#]].assert_debug_eq(red_tree.tree());
```

</details>

## `LexerStream` Implementation

### What's Different?

The main difference required on our end to be able to use the full expressive power of [`logos`] is, well, [`logos::Logos`]. We need a type that can serve as lexer output, containing all of the distinct terminals that this input may contain.

Wouldn't you know it, that is exactly what `Kind` is! We simply need to derive [`logos::Logos`]:

```rust
##[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, logos::Logos)]
##[repr(u16)]
##[logos(skip(r"[\s]+"))]        // <--
enum Kind {
    // Terminals.
    Whitespace = 0,
    #[token("(")]               // <--
    LeftParen,
    #[token(")")]               // <--
    RightParen,
    #[regex("[a-zA-Z0-9_-]+")]  // <--
    Atom,

    // Non-terminals
    List,
    Root,
}
```

Note that the attribute for whitespace skipping is placed at the top of the enum, and `logos` itself does not construct this type. `Kind::Whitespace` is instead supplied to the wrapping [`LexerStream`] when constructed for any form of trivia skipped.

The main capability this unlocks for us is direct `Kind` matching (given [`Eq`] is implemented). Taking our previous `atom` example:

```rust,ignore
let atom = just::<'parse, SkippingStr<'source>, E, _>(
    (Kind::Atom, regex::Regex::new("[a-zA-Z0-9_-]+").unwrap())
);
```

We can narrow it down to:

```rust,ignore
let atom = just::<'parse, LexerStream<'source, Kind>, E, _>( WithText(Kind::Atom) );
```

Note that with this pattern form, there is now no way to mix up a [`rowan::SyntaxKind`] from its associated matching type. A `Kind::Atom` *must* correspond to an alphanumeric terminal. This is the biggest benefit and drawback of a global lexing pass.

[`WithText`] returns both the token kind matched and the underlying source string as [`(&Kind, &str)`](primitive@tuple). This means that the fragment within the `sexp` parser that transforms it into an `SExp` value must also be modified:

```rust
# use biparse::{
#     prelude::*,
#     input::skipping_str::SkippingStr,
# };
# use expect_test::expect;
# 
# #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, logos::Logos)]
# #[repr(u16)]
# #[logos(skip(r"[\s]+"))]
# enum Kind {
#     // Terminals.
#     Whitespace = 0,
#     #[token("(")]
#     LeftParen,
#     #[token(")")]
#     RightParen,
#     #[regex("[a-zA-Z0-9_-]+")]
#     Atom,
# 
#     // Non-terminals
#     List,
#     Root,
# }
# impl rowan::Language for Kind {
#     type Kind = Self;
#     fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
#         match raw.0 {
#             0 => Kind::Whitespace,
#             1 => Kind::LeftParen,
#             2 => Kind::RightParen,
#             3 => Kind::Atom,
#             4 => Kind::List,
#             5 => Kind::Root,
#             _ => unreachable!(),
#         }
#     }
#     fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
#         rowan::SyntaxKind(
#             match kind {
#                 Kind::Whitespace => 0,
#                 Kind::LeftParen => 1,
#                 Kind::RightParen => 2,
#                 Kind::Atom => 3,
#                 Kind::List => 4,
#                 Kind::Root => 5,
#             }
#         )
#     }
# }
# #[allow(clippy::from_over_into)]
# impl Into<rowan::SyntaxKind> for Kind {
#     fn into(self) -> rowan::SyntaxKind {
#         <Kind as rowan::Language>::kind_to_raw(self)
#     }
# }
# 
# #[derive(Debug, Clone, PartialEq, Eq)]
# enum SExp {
#     Atom(String),
#     List(Vec<SExp>),
# }
#
// Signature changed!
fn sexp_parser<'source, 'parse, E: ParseEnvironment>() -> impl Parser<'parse, LexerStream<'source, Kind>, SExp, E>
where 'source : 'parse {

    // changed!
    let atom =just::<'parse, LexerStream<'source, Kind>, E, _>(WithText(Kind::Atom));

    Recursive::new(|sexp|
        atom.map(|(_kind, s)| SExp::Atom(s.to_owned()))  // <-- changed!
            .named("atom", Kind::Atom)
        .or(
            just(Kind::LeftParen)                        // <-- simplified.
            .ignore_then(
                sexp.many().map(SExp::List)
            ).then_ignore(just(Kind::RightParen))        // <-- simplified.
            .named("list", Kind::List)
        )
    )
}
```

This parser behaves identically to that made by [`SkippingStr`](#skippingstr-implementation).