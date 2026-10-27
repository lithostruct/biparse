# Parse Output and Outcomes

The intermediate output of a parse, `ParseOutput`, consists of an AST and CST component. 
Both components are vital to the functionality of the `biparse` combinator library.

## Abstract Syntax Tree (AST)

The AST component consists of a `ParseOutcome`, which has the definition:
```rs,ignore
/// Status of parse result of the AST payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseOutcome<A, F> {
    /// A successful parse producing an AST payload.
    Success(A),
    /// Speculative failure.
    /// The combinator system is free to try out parsing alternatives,
    /// and a later success will cause this value to be discarded.
    Speculative(F),
    /// Committed failure.
    /// The combinator system may not try other alternatives.
    /// This failure is propagated outwards until explicitly handled.
    Committed(F),
}
```
- Successful parse results in the AST value `A`. These may be formed and transformed via input matching, composition and mapping.
- Speculative failure is the default failure mode of the combinator ecosystem. This indicates while the current parse branch failed, alternatives may be tried, and success within sister or enclosing branches will be taken as success overall.  
  This allows for speculative parsing via `or`, repetition via `many`, and recursive parsing via `Recursive` combinators.
- Committed failure indicates that speculative parsing should *not* be attempted. This usually results from explicit commitment points via `commit`, where sufficient syntax is recognized to commit to a specific parsing branch, or from fallible transformation of the success value via `try_map`.  
  Commited failure follows exception semantics, and propagate outwards until explicitly handled using an enclosing `recover` combinator.

The type of failures, `F` may be configured by implementing the `ParseEnvironment` threaded throughout a parse operation. This allows the end user to control how much failure information to retain for error reporting purposes.

As a simplistic but illustrative example regarding AST outcome semantics:
- The primitive `success()` succeeds for anything, returning `()`.
- `a.then(b)` succeeds for `"ab"`, returning `(A, B)`.
- `a.or(b)` succeeds on either `"a"` or `"b"`, returning an `Either<A, B>` AST type. It returns a speculative failure on `c`,
  but `c` does succeed on `a.or(b).or(c)` (which has the AST type `Either<Either<A, B>, C>`).
- `a.commit().or(b)` returns a committed failure on anything that is not `"a"`. In fact, the `b` parser won't be tried at all.  
  `(a.or(b)).commit().or(c)` succeeds on either `"a"` or `"b"`. Anything after commitment to `a.or(b)` fails (including `"c"`!).
- Given `some_string` successfully parses a string as a `&str`, then `some_string.try_map(|s| u32::from_str(s))` will result in a committed failure on `"Not a number!"`; there is simply no appropriate value of type `u32` to convert it to.

The `parse_all` and `check_all` top-level parsing methods lift parse failure out of the AST component, producing `Result<ParseArtifact<...>, F>`, where `ParseArtifact` is the AST and CST components of a successful parse. In other words, parsers that fail to parse the input as a whole is considered to be meaningless, the CST is not retained, and the offending failure that surfaces to the top-level is reported.

## Concrete Syntax Tree (CST)

The CST component, `AssociatedGreenCst` consists of a `rowan` CST forest, along with coupled path information for both associations and diagnostics. The structure of `AssociatedGreenCst` can be thought of as:
```rs,ignore
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
- The `rowan` CST forest is constructed automatically during parsing. `Parser` implementors maintain the extension contract that the CST formed is total.
- Associations are introduced by `associate` combinators, which allow a unique `Id` value to be embedded into both the AST and CST.
- Diagnostics are formed from successful error recovery from committed failures via `recover` combinators. This causes a unique `EId` value to be embedded into both the AST and CST.

The `Id` and `EId` associated types may be configured by implementing the `ParseEnvironment` threaded throughout a parse operation. The default types for both associated types is `usize`.

`AssociatedGreenCst` maintains the strong invariant that all association and diagnostic paths correspond to the same green nodes within the CST forest throughout CST transformation for the entire parse operation. This also means that `Id` and `EId` values embedded within the AST constructed will always point to the same CST spans from which they are associated with. This forms the AST-CST correspondence mechanism of `biparse`.

`AssociatedGreenCst` may be canonized into `AssociatedRedCst`, which instead contains a `rowan::SyntaxNode<L>` red tree. Both types may weaken their correspondence invariants via deconstruction to take ownership of their constituent parts.