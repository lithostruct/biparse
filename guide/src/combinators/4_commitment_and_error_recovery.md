# Commitment and Error Recovery

This section relates exclusively to [`Committed`](./../parse_output.md/#committed) failure recovery. Refer to [Parse Output and Outcomes](./../parse_output.md) for the distinction between speculative failure and committed failure.

For speculative failure, trying an alternative parsing branch may be constituted its own form of recovery, and an `.or(success())` catch-all may be sufficient.

## Commitment Boundaries

### `commit`

Promotes a speculative failure into a committed failure. This is performed at commitment points where sufficient syntax is recognized to be unambiguous and hence not attempt other parsing paths; eg. recognizing the `let` keyword and expecting the syntax rule `let_binding ::= let identifier [':' type] '=' expression ';'`.

### `then_commit_to`

Syntactic sugar over `a.then(b.commit())`, which resembles a Prolog-like cut boundary.

Note that eg. `a.then_commit_to(b).then(c)` will only enforce commitment in constituent `b`; `c` may fail speculatively if `a` and `b` succeed, returning a speculative error. If `b` fails, it returns a committed failure, and `c` is not tried at all.

## Recovery Parsing

The `recover` combinator takes in:
- The upstream combinator via method chaining,
- A recovery parser,
- A closure that is invoked upon successful recovery, taking in the recovery payload and an `Id` value supplied by `ParseEnvironment::ErrorSupply::Id`, and returning the transformed or embedded AST value.

Recovery parsers implement a recovery strategy. Common strategies include:
- Discarding all text until the end-of-input.
- Discarding all text up to (and potentially including) a set of recognized "checkpoint" characters.

Recovery strategies can be as elaborate as regular parsers, but are typically kept simple for robustness. A recovery parser that fails (whether speculative or committed) will cause the initial failure to be propagated outwards.

## Diagnostic Generation

Upon recovery, a committed failure is converted into a diagnostic. Diagnostics are associated with specific CST nodes, and can be embedded via identification values. You can think of recovery success as also performing an [`associate`](./3_transformation.md/#associate)-like operation onto the AST and CST.

It is the common case that subsequent analysis of the parse artifact should not be attempted if diagnostics are present (as diagnostics typically represent parse or conversion failure). As such, the top-level `ParseArtifact` bundles together diagnostics in the accessor function `ParseArtifact::ast`.