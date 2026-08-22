use std::{borrow::Cow, fmt::Debug, rc::Rc};

use crate::{
    cst::{CstOutput, IdSupply},
    failure::{FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};

/// The recursive stack stored in `ParseState`, which can be cheaply cloned.
///
/// Internals hidden due to being specific to `Recursive` and no other combinator.
pub struct RecursiveStack<'src, I: Input, S: IdSupply> {
    inner: im::Vector<RecursiveFrame<'src, I, S>>,
}
impl<'src, I: Input, S: IdSupply> Default for RecursiveStack<'src, I, S> {
    fn default() -> Self {
        RecursiveStack {
            inner: im::Vector::new(),
        }
    }
}
impl<'src, I: Input, S: IdSupply> Clone for RecursiveStack<'src, I, S> {
    fn clone(&self) -> Self {
        RecursiveStack {
            inner: self.inner.clone(),
        }
    }
}
impl<'src, I: Input, S: IdSupply> RecursiveStack<'src, I, S> {
    pub fn new() -> Self {
        Self::default()
    }
    fn get_latest(
        &self,
        id: RecursiveId,
        start_cursor: I::Cursor,
    ) -> Option<&RecursiveFrame<'src, I, S>> {
        self.inner
            .iter()
            .rev()
            .find(|inner| inner.matches(id, start_cursor))
    }
    fn push_to_top(&self, frame: RecursiveFrame<'src, I, S>) -> Self {
        RecursiveStack {
            inner: self.inner.clone() + im::Vector::unit(frame),
        }
    }
}

struct RecursiveFrame<'src, I: Input, S: IdSupply> {
    // Identifying information
    id: RecursiveId,
    start_cursor: I::Cursor,

    // Diagnostic information.
    description: Cow<'static, str>,

    // Payload.
    /// This cannot be in `erased` becaue `I` does not live for `'static`.
    seed_result: Option<(ParseState<'src, I, S>, Rc<dyn ErasedOutput<'src> + 'src>)>,
}
impl<'src, I: Input, S: IdSupply> Debug for RecursiveFrame<'src, I, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecursiveFrame")
            .field("description", &self.description)
            .field("erased", &"<erased>")
            .finish()
    }
}
impl<'src, I: Input, S: IdSupply> Clone for RecursiveFrame<'src, I, S> {
    fn clone(&self) -> Self {
        RecursiveFrame {
            id: self.id,
            start_cursor: self.start_cursor,
            description: self.description.clone(),
            seed_result: self.seed_result.clone(),
        }
    }
}
impl<I: Input, S: IdSupply> RecursiveFrame<'_, I, S> {
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn matches(&self, id: RecursiveId, start_cursor: I::Cursor) -> bool {
        self.id == id && self.start_cursor == start_cursor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RecursiveId(*const ());

/// Trait erasure for storing heterogeneous [`ParseOutput<A, E>`] within [`RecursiveFrame`].
trait ErasedOutput<'src> {
    fn clone_erased(&self) -> Rc<dyn ErasedOutput<'src> + 'src>;
    fn as_ptr(&self) -> *const ();
}
impl<'src, A, E: ParseEnvironment> ErasedOutput<'src> for ParseOutput<A, E>
where
    A: Clone + 'src,
{
    fn clone_erased(&self) -> Rc<dyn ErasedOutput<'src> + 'src> {
        Rc::new((*self).clone())
    }
    fn as_ptr(&self) -> *const () {
        self as *const Self as *const ()
    }
}

#[derive(Debug, Clone)]
pub struct Recursive<F> {
    description: Cow<'static, str>,
    closure: F,
}
impl<F> Recursive<F> {
    /// This is only meaningful within a parse session, as `&self` is reborrowed and thus maintains its pointer address.
    #[inline]
    fn id(&self) -> RecursiveId {
        RecursiveId(self as *const Self as *const ())
    }
}
impl<'src, I: Input, A, E: ParseEnvironment, F, P> Parser<'src, I, A, E> for Recursive<F>
where
    F: Fn(&Self) -> P,
    P: Parser<'src, I, A, E>,
    A: Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>) {
        match state.recursive_stack.get_latest(self.id(), state.cursor) {
            Some(frame) => {
                // encountered. check for seed?
                match &frame.seed_result {
                    // clone seed value, restore its type and return.
                    Some((seed_state, erased)) => {
                        assert_eq!(
                            frame.id,
                            self.id(),
                            "recursive output belongs to a different Recursive combinator!"
                        );
                        // # SAFETY:
                        //
                        // Every `RecursiveFrame` is associated with exactly one `RecursiveId`,
                        // identifying the Recursive combinator whose invocation the frame represents.
                        //
                        // Assume the invariant that construction of `RecursiveFrame`
                        // stores the payload and RecursiveId of the combinator that produced it,
                        // which must not be decoupled after erasure within this parse invocation.
                        //
                        // The erased payload is asserted to be created by this Recursive combinator,
                        // hence the output must be of type `ParseOutput<A, E>`.
                        //
                        // `ErasedOutput::as_ptr` points directly to its contained `ParseOutput<A, E>`.
                        // We borrow that value and clone it; ownership of the erased output remains unchanged.
                        let parse_output = unsafe {
                            let output = &*(erased.as_ptr() as *const ParseOutput<A, E>);
                            output.clone()
                        };
                        (seed_state.clone(), parse_output)
                    }
                    None => {
                        // speculative error for left-recursion.
                        let failure = E::Failure::failure_at::<_, E::Failure>(
                            input,
                            state.cursor,
                            FailureKind::LeftRecursion {
                                description: self.description.clone().into_owned(),
                            },
                        );
                        (
                            state,
                            ParseOutput {
                                ast: ParseResult::Speculative(failure),
                                cst: CstOutput::new(),
                            },
                        )
                    }
                }
            }
            None => {
                // first encounter.
                let mut recursive_stack = state.recursive_stack.push_to_top(RecursiveFrame {
                    id: self.id(),
                    start_cursor: state.cursor,
                    description: self.description.clone(),
                    seed_result: None,
                });
                let seed_state = ParseState {
                    recursive_stack: recursive_stack.clone(),
                    ..state.clone()
                };
                // Make the Ouroboros.
                let parser = (self.closure)(self);

                // Check if the initial sprout encountered failures.
                // If yes, there is no point in seed growing, and the failure should be propagated.
                let (sprout_state, sprout_result) = parser.parse(input, seed_state);
                match &sprout_result.ast {
                    ParseResult::Speculative(_) | ParseResult::Committed(_) => {
                        // failure to sprout.
                        return (sprout_state, sprout_result);
                    }
                    // Succcessful seed.
                    // Any speculative errors now would be an indication to halt.
                    // Committed errors should still be propagated though.
                    ParseResult::Success(_) => (),
                }

                let (mut rec_seed_state, mut rec_seed_result) = (sprout_state, sprout_result);
                loop {
                    // Use the original `state` recursive stack to "replace" this combinator's frame every invocation.
                    //
                    // # SAFETY
                    //
                    // Invariant regarding `RecursiveFrame` construction fulfilled;
                    // This is the only location where it is constructed.
                    recursive_stack = state.recursive_stack.push_to_top(RecursiveFrame {
                        id: self.id(),
                        start_cursor: state.cursor,
                        description: self.description.clone(),
                        seed_result: Some((rec_seed_state.clone(), rec_seed_result.clone_erased())),
                    });
                    let next_seed_state = ParseState {
                        recursive_stack,
                        ..state.clone()
                    };
                    let (next_sprout_state, next_sprout_result) =
                        parser.parse(input, next_seed_state);
                    match &next_sprout_result.ast {
                        ParseResult::Success(_) => {
                            // Grow the seed and continue, if not zero-progress.
                            if rec_seed_state.cursor != next_sprout_state.cursor {
                                rec_seed_state = next_sprout_state;
                                rec_seed_result = next_sprout_result;
                                continue;
                            } else {
                                // Zero progress encountered.
                                // Treat stagnancy similarly to speculative error:
                                // reached fixpoint and return.
                                return (rec_seed_state, rec_seed_result);
                            }
                        }
                        ParseResult::Speculative(_) => {
                            // The previous iteration has consumed as much as we can without speculative failure.
                            // It's time to stop.
                            return (rec_seed_state, rec_seed_result);
                        }
                        ParseResult::Committed(_) => {
                            // Propagate failure.
                            return (next_sprout_state, next_sprout_result);
                        }
                    }
                }
            }
        }
    }
}
