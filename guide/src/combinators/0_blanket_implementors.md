# Blanket Implementors

`Parser` is blanket-implemented for the following types where `P : Parser<...>`:
- `&P`.
- `Box<P>`. May be constructed via `.boxed()`.
- `Rc<P>`. May be constructed via `.shared()`.
- `Arc<P>`.

## `With<E, P>`

Transparent wrapper that fixes the inner parser type `P` to use the given `E: ParseEnvironment`.
Defers to the inner implementation.

## `As<'src, I, A, E, P>`

Transparent wrapper that fixes the inner parser type `P` to act as `Parser<'src, I, A, E>`.
Typically used for leaf combinators. Defers to the inner implementation.