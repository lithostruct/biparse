# Commitment and Error Recovery

This section relates exclusively to [`Committed`](./../parse_output.md#committed) failure recovery. Refer to [Parse Output and Outcomes](./../parse_output.md) for the distinction between speculative failure and committed failure.

For speculative failure, trying an alternative parsing branch may be constituted its own form of recovery, and an `.or(success())` catch-all may be sufficient.

## Commitment Boundaries

### `commit`

Promotes a speculative failure ([`ParseOutcome::Speculative`](../parse_output.md#speculative)) into a [`ParseOutcome::Committed`](../parse_output.md#committed) failure.

This may be performed at commitment points where sufficient syntax is recognized to be unambiguous and hence not attempt other parsing paths; eg. recognizing the `let` keyword and expecting the unique syntax rule:
```text
let_binding ::= "let" identifier [':' type] '=' expression ';'
```

### `then_commit_to`

Syntactic sugar over `a.then(b.commit())`, which resembles a Prolog-like cut boundary.

Reusing the `let_binding` example:
```text
let_binding ::= "let" ! identifier [':' type] '=' expression ';'
                      ^ cut boundary. failure beyond this point should be committed.
```
may be translated to a first approximation (ignoring typing of AST payload) as:
```rust,ignore
// assumed that `E: ParseEnvironment` is in-scope,
// and that `I: Input` is whitespace-insensitive
// and accepts character and string patterns:

let let_binding : Rc<dyn Parser<'_, I, _, E>> = just((/* kind */, "let"))
    .then_commit_to(
        identifier
            .then(just(/* kind */, ':').then(type_expression).optional())
            .then(just(/* kind */, '='))
            .then(expression)
            .then(just(/* kind */, ';'))
    )
    .shared();
```

Note that eg. `a.then_commit_to(b).then(c)` will only enforce commitment in constituent `b`; `c` may fail speculatively if `a` and `b` succeed, returning a speculative error. If `b` fails, it returns a committed failure, and `c` is not tried at all.

## Recovery Parsing (`recover`)

The `recover` combinator takes in:
- The upstream combinator via method chaining,
- A recovery parser,
- A closure that is invoked upon successful recovery, taking in the recovery payload and an `Id` value supplied by `ParseEnvironment::ErrorSupply::Id`, returning the transformed or embedded AST value.

Recovery parsers implement a recovery strategy. Common strategies include:
- Discarding all text until the end-of-input.
- Discarding all text up to (and potentially including) a set of recognized "checkpoint" tokens.

Recovery parsers can be as elaborate as regular parsers (and indeed are represented as parser objects!), but are typically kept simple for robustness. A recovery parser that fails will cause the propagation of the initial failure to resume.

## Diagnostic Generation

Upon successful recovery, a committed failure is converted into a diagnostic. Diagnostics are associated with specific CST nodes, and can be embedded via identification values. You can think of recovery success as performing an [`associate`](./3_transformation.md#associate)-like operation onto the AST and CST.

Diagnostics typically represent recovered parse errors that nonetheless represents malformed input. It is the common case that subsequent analysis of the parse artifact should not be attempted if diagnostics are present.

As such, the top-level `ParseArtifact` bundles diagnostics together with the AST payload in accessor and conversion methods `ParseArtifact::ast` and `ParseArtifact::into_ast`.
To explicitly ignore or discard diagnostics, use `ParseArtifact::ast_only` and `ParseArtifact::into_ast_only`.