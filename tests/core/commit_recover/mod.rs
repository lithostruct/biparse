//! Testcases for `commit`, `recover` and `try_map`.
//! These operations are grouped into a dedicated module because all these combinators
//! interact with commitment semantics, which exhibit exception-like unwinding control flow.

mod commit;
mod recover;
mod try_map;
