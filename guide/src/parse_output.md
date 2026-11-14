# Parse Output and Outcomes

The intermediate output of a parse, `ParseOutput`, consists of an AST and CST component. 
Both components are vital to the functionality of the Biparse combinator library.

## Abstract Syntax Tree (AST)

The AST component consists of a `ParseOutcome`. You may think of this as a `Result<A, F>` payload with subcategories describing how the parser ecosystem should react to failure, in increasing levels of severity.

The `parse_all` and `check_all` top-level parsing methods lift parse failure out of the AST component, producing `Result<ParseArtifact<...>, _>`, where `ParseArtifact` is the AST and CST components of a successful parse. In other words, parsers that fail to parse the input as a whole is considered to be meaningless, the CST is not retained, and the offending failure that surfaces to the top-level is reported.

The type of failures may be configured by changing the `ParseEnvironment` threaded throughout a parse operation. This allows the end user to control how much failure information to retain for error reporting purposes.

Three failure payload types are provided:
- `Minimal`, which discards all error information.
- `PositionOnly`, which retains the 

### `Success`

Successful parse results in the AST value `A`. These may be constructed and transformed via input matching, composition and mapping.

### `Speculative`

Speculative failure is the default failure mode of the combinator ecosystem. This indicates while the current parse branch failed, alternatives may be tried, and success within sister or enclosing branches will be taken as success overall. This allows for speculative parsing via `or`, repetition via `many`, and recursive parsing via `Recursive` combinators.

### `Committed`

Committed failure indicates that speculative parsing should *not* be attempted. This usually results from explicit commitment points via `commit`, where sufficient syntax is recognized to commit to a specific parsing branch, or from fallible transformation of the success value via `try_map`.

Committed failure follows exception semantics, and propagate outwards until explicitly handled using an enclosing `recover` combinator. Upon recovery, committed failures are transformed into [diagnostics](#concrete-syntax-tree-cst).

### `Fatal`

Fatal failures arise from ecosystem invariant violations, including but not limited to:
- Non-growing repetition in repetition composites such as `many` and `some`. [(read more)](./combinators/2_composite_combinators.md/#repetition)
- Unbounded recursion in recursive constructs eg. `expr ::= expr`.
- Undefined parsers from `Recursive::declare`. [(read more)](./combinators/5_recursive_combinators.md/#mutual-recursion)

Fatal failures are unrecoverable. They represent inherent errors in the construction of the parser object.

### Examples

As a simplistic but illustrative example regarding AST outcome semantics:
- The primitive `success()` succeeds for anything, returning `()`.
- `a.then(b)` succeeds for `"ab"`, returning `(A, B)`.
- `a.or(b)` succeeds on either `"a"` or `"b"`, returning an `Either<A, B>` AST type. It returns a speculative failure on `c`,
  but `c` does succeed on `a.or(b).or(c)` (which has the AST type `Either<Either<A, B>, C>`).
- `a.commit().or(b)` returns a committed failure on anything that is not `"a"`. In fact, the `b` parser won't be tried at all.  
  `(a.or(b)).commit().or(c)` succeeds on either `"a"` or `"b"`. Anything after commitment to `a.or(b)` fails (including `"c"`!).
- Given `some_string` successfully parses a string as a `&str`, then `some_string.try_map(|s| u32::from_str(s))` will result in a committed failure on `"Not a number!"`; there is simply no appropriate value of type `u32` to convert it to, but enclosing parsers producing an `Option<u32>` can recover and produce the valid AST payload `None`.
- `success().many()` produces a fatal failure, as `success()` consumes no input and hence how many items to populate the payload `Vec<()>` with is undecidable. A naive implementation would spin forever.

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

`AssociatedGreenCst` maintains the strong invariant that all association and diagnostic paths correspond to the same green nodes within the CST forest throughout CST transformation for the entire parse operation. This also means that `Id` and `EId` values embedded within the AST constructed will always point to the same CST spans from which they are associated with. This forms the AST-CST correspondence mechanism of Biparse.

`AssociatedGreenCst` may be canonized into `AssociatedRedCst`, which instead contains a `rowan::SyntaxNode<L>` red tree. Both types may weaken their correspondence invariants via deconstruction to take ownership of their constituent parts.