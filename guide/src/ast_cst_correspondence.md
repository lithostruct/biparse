# AST-CST Correspondence

The `Id` type supplied corresponds to `<E as ParseEnvironment>::Suppply::Id`, which may be changed by implementing `ParseEnvironment` for a zero-sized type. The default type is a `usize`.

## Invariants of CST Correspondence

We assume the extension contract of `IdSupply` to be upheld ie. the `Id`s produced are all distinct.

Left-right contatenation of CST forests hence forms a monoid:
\\[ (M, ⧺, \epsilon) \\]

This implies the following laws:

| Case | Expected Result | Monoid Notation |
|:-----|:----------------|:---------------:|
| Both forests empty   | Empty forest         | \\( \epsilon ⧺ \epsilon = \epsilon \\) |
| Left forest empty    | Right forest only    | \\( \epsilon ⧺ A = A \\)               |
| Right forest empty   | Left forest only     | \\( A ⧺ \epsilon = A \\)               |
| Single concatenation | Left unchanged, right shifted | \\( [ a ] ⧺ [ b ] = [ a , b ] \\) |
| Nested concatenation | Associative          | \\( A ⧺ (B ⧺ C) = (A ⧺ B) ⧺ C \\)     |