# Commitment and Error Recovery

This section relates exclusively to committed failure recovery. Refer to [Parse Output and Outcomes](./../parse_output.md) for the distinction between speculative failure and committed failure. For speculative failure, trying an alternative parsing branch may be constituted its own form of recovery, and an `.or(success())` catch-all may be sufficient.

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

Upon recovery, a committed failure is converted into a diagnostic. Diagnostics are associated with specific CST nodes, and can be embedded via identification values. You can think of recovery success as also performing an `associate`-like operation onto the AST and CST.

It is the common case that subsequent analysis of the parse artifact should not be attempted if diagnostics are present (as diagnostics typically represent parse or conversion failure). As such, the top-level `ParseArtifact` bundles together diagnostics in the accessor function `ParseArtifact::ast`.