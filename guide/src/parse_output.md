# Parse Outcomes, Output and Artifacts

[`ParseOutput`] refer to the intermediate output of a parse, which consists of an AST and CST component. 
Both components are vital to the functionality of the Biparse combinator library. It is intended to be used primarily by [`Parser`](./parser_trait.md#main-parser-trait) implementors and those that interact with [`Parser`] functionality.

[`ParseArtifact`] are finalized from [`ParseOutput`], containing the same AST and CST information but in a more ergonomic form for end-users. They are produced by the top-level [`ParserExt::parse_all`] and [`ParserExt::check_all`] methods.

[`ParseOutcome`] refers to the AST component of both output types. More is covered in its [dedicated section](#abstract-syntax-tree-ast).

## Abstract Syntax Tree (AST)
*Docs.rs documentation: [`ParseOutcome`].*

The intermediate AST component consists of a [`ParseOutcome`]. You may think of this as a [`Result<A, F>`] payload with subcategories describing how the parser ecosystem should react to failure, in increasing levels of severity.

Finalization from [`ParseOutput`] to [`ParseArtifact`] lossily converts parse outcome into the corresponding `Result`, lifting failure out of the AST component to indicate overall success or failure. This means that the event of a top-level failure, the CST is *not* retained.

The type of failures may be configured by changing the [`ParseEnvironment`] threaded throughout a parse operation. This allows the end user to control how much failure information to retain for error reporting purposes.

Three failure payload types are provided, implementing the necessary [`ParseFailure`] trait:
- [`Minimal`](biparse::failure::types::Minimal), which discards all error information.
- [`Simple`](biparse::failure::types::Simple), which retains the line-column position and simplistic failure kind information.
- [`Rich`](biparse::failure::types::Rich), which retains and pretty-prints most of the information supplied at the site of failure.

These may be accessed through module [`failure::types`]. They are not exported via the [`prelude`].

### `Success`

Successful parse results in the AST value `A`. These may be constructed and transformed via input matching, composition and mapping.

### `Speculative`

Speculative failure is the default failure mode of the combinator ecosystem. This indicates while the current parse branch failed, alternatives may be tried, and success within sister or enclosing branches will be taken as success overall. This allows for speculative parsing via `or`, repetition via `many`, and recursive parsing via [`Recursive`] combinators.

### `Committed`

Committed failure indicates that speculative parsing should *not* be attempted. This usually results from explicit commitment points via `commit`, where sufficient syntax is recognized to commit to a specific parsing branch, or from fallible transformation of the success value via `try_map`.

Committed failure follows exception semantics, and propagate outwards until explicitly handled using an enclosing `recover` combinator. Upon recovery, committed failures are transformed into [diagnostics](#concrete-syntax-tree-cst).

### `Fatal`

Fatal failures arise from ecosystem invariant violations, including but not limited to:
- Non-growing repetition in repetition composites. [(read more)](./combinators/2_composite_combinators/23_repetition.md#repeat_by)
- Unbounded recursion in recursive constructs eg. `self ::= self`.
- Undefined parsers from `Recursive::declare`. [(read more)](./combinators/5_recursive_combinators.md#mutual-recursion)

Fatal failures are unrecoverable, and *should not* be recovered from.

### Examples

As a simplistic but illustrative example regarding AST outcome semantics:
- The primitive `success()` succeeds for anything, returning `()`.
- `a.then(b)` succeeds for some input `ab`, returning `(A, B)`. Having input containing `a` but not `b` returns speculative failure; both are required for success.
- `a.or(b)` succeeds on either `a` or `b`, in that order. If `a` fails speculatively, `b` is then tried.
- `a.commit().or(b)` returns a committed failure on anything that is not `a`.  
  In fact, the `b` parser won't be tried at all!
- Given `some_string` successfully parses a string as a `&str`, then `some_string.try_map(|s| u32::from_str(s))` will result in a committed failure on `"Not a number"`; there is simply no appropriate value of type `u32` to convert it to.  
  Enclosing parsers producing an `Option<u32>` can choose to recover and produce the valid AST payload `None`.
- `success().many()` produces a fatal failure, as `success()` consumes no input and hence how many items to populate the payload `Vec<()>` with is undecidable. A naive implementation would spin forever.

## Concrete Syntax Tree (CST)
*Docs.rs documentation: [`AssociatedGreenCst`], [AssociatedRedCst].*

The CST component [`AssociatedGreenCst`] consists of a `rowan` CST forest, along with coupled path information for both associations and diagnostics. The structure of [`AssociatedGreenCst`] can be thought of as:
```rust,ignore
pub struct AssociatedGreenCst<Id, EId, E>
where
    Id: Eq + Hash,
    EId: Eq + Hash,
{
    cst: Vec<NodeOrToken<GreenNode, GreenData>>,
    associations: HashMap<Id, CstPath>,
    diagnostics: HashMap<EId, (E, CstPath)>,
}
```
- The [`rowan`] CST forest is constructed automatically during parsing. [`Parser`] implementors maintain the extension contract that the CST formed is total.
- Associations are introduced by [`associate`](./combinators/3_transformation.md#associate) combinators, which allow a unique `Id` value to be embedded into both the AST and CST.
- Diagnostics are formed from successful error recovery from committed failures via [`recover`](./combinators/4_commitment_and_error_recovery.md#recovery-parsing-recover) combinators. This causes a unique `EId` value to be embedded into both the AST and CST.

The `Id` and `EId` associated types may be configured by implementing the [`ParseEnvironment`] threaded throughout a parse operation. The default for both associated types is `usize`.

[`AssociatedGreenCst`] maintains the strong invariant that all association and diagnostic paths correspond to the same green nodes within the CST forest throughout CST transformation for the entire parse operation. This also means that `Id` and `EId` values embedded within the AST constructed will always point to the same CST spans from which they are associated with. This forms the AST-CST correspondence mechanism of Biparse.

[`AssociatedGreenCst`] may be canonized into [`AssociatedRedCst`], which instead contains a [`rowan::SyntaxNode<L>`] red tree. Both types may weaken their correspondence invariants via deconstruction to take ownership of their constituent parts.