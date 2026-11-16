# Blanket Implementors

`Parser` is blanket-implemented for the following types where `P : Parser<...>`:
- `&P`.
- `Box<P>`. May be constructed via `.boxed()`.
- `Rc<P>`. May be constructed via `.shared()`.
- `Arc<P>`.

## `As<'src, I, A, E, P>`

Transparent wrapper that fixes the inner parser type `P` to act as `Parser<'src, I, A, E>`.
Typically used to constrain the `Parser` implementation of [primitives and leaf combinators](./1_primitives_and_leaf_combinators.md). Defers to the inner implementation.