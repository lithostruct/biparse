use std::{
    borrow::Cow,
    cell::OnceCell,
    fmt::Debug,
    rc::{Rc, Weak},
};

use crate::{
    combinator::recursive::frame::{ErasedOutput, RecursiveFrame, RecursiveId},
    cst::CstOutput,
    failure::{FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseOutput, ParseResult, ParseState, Parser},
};

// The following structures are basically copied from `chumsky`'s approach to their `Recursive` combinator.

#[derive(Debug, Clone)]
pub struct Recursive<P: ?Sized> {
    /// Inner payload.
    inner: RecursiveInner<P>,
    /// Description.
    description: Option<Cow<'static, str>>,
}
impl<P: ?Sized> Recursive<P> {
    fn recursive_id(&self) -> RecursiveId {
        self.inner.recursive_id()
    }
    fn diagnostic_name(&self) -> String {
        format!("{:p}", self.inner.as_ptr() as *const ())
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment> Recursive<Direct<'src, I, A, E>> {
    #[allow(clippy::self_named_constructors)]
    pub fn recursive<F, P>(closure: F) -> Self
    where
        F: FnOnce(Recursive<Direct<'src, I, A, E>>) -> P,
        P: Parser<'src, I, A, E> + 'src,
    {
        let rc: Rc<P> = Rc::new_cyclic(|weak| {
            let this: Recursive<Direct<'src, I, A, E>> = Recursive {
                inner: RecursiveInner::Unowned(weak.clone() as Weak<Direct<'src, I, A, E>>),
                description: None,
            };
            closure(this)
        });
        Recursive {
            inner: RecursiveInner::Owned(rc as Rc<Direct<'src, I, A, E>>),
            description: None,
        }
    }

    pub fn recursive_as<F, P>(s: impl Into<Cow<'static, str>>, closure: F) -> Self
    where
        F: FnOnce(Recursive<Direct<'src, I, A, E>>) -> P,
        P: Parser<'src, I, A, E> + 'src,
    {
        let description = s.into();
        let rc: Rc<P> = Rc::new_cyclic(|weak| {
            let this: Recursive<Direct<'src, I, A, E>> = Recursive {
                inner: RecursiveInner::Unowned(weak.clone() as Weak<Direct<'src, I, A, E>>),
                description: Some(description.clone()),
            };
            closure(this)
        });
        Recursive {
            inner: RecursiveInner::Owned(rc as Rc<Direct<'src, I, A, E>>),
            description: Some(description),
        }
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment> Recursive<Indirect<'src, I, A, E>> {
    pub fn declare() -> Self {
        Recursive {
            inner: RecursiveInner::Owned(Rc::new(Indirect {
                description: None,
                inner: OnceCell::new(),
            })),
            description: None,
        }
    }
    pub fn declare_as(s: impl Into<Cow<'static, str>>) -> Self {
        Recursive {
            inner: RecursiveInner::Owned(Rc::new(Indirect {
                description: Some(s.into()),
                inner: OnceCell::new(),
            })),
            description: None,
        }
    }
    /// # Note on Strong Cycles
    ///
    /// Mutually-recursive groups contain strong references into each other;
    /// hence they will never deallocate even after becoming inaccessible.
    ///
    /// This is currently considered acceptable because parsers typically are constructed no more than once,
    /// and stored and resused throughout the lifetime of the program.
    ///
    /// Future mitigation of this would entail a `RecursiveGroup`-like structure involving n-variadic type arguments.
    pub fn define<P>(&mut self, parser: P)
    where
        P: Parser<'src, I, A, E> + 'src,
    {
        match &mut self.inner {
            RecursiveInner::Owned(owned) => owned
                .as_ref()
                .inner
                .set(Box::new(parser))
                .unwrap_or_else(|_| {
                    panic!("Called Recursive::define on already-defined combinator{}!",
                        self.description.as_ref().map(|s| format!(" '{s}'")).unwrap_or("".into()))
                }),
            RecursiveInner::Unowned(_) => {
                panic!("Called Recursive::define on weak reference{}!",
                    self.description.as_ref().map(|s| format!(" '{s}'")).unwrap_or("".into()))
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RecursiveInner<P: ?Sized> {
    // Owned. What users get during direct construction, declaration and definition.
    Owned(Rc<P>),
    // Unowned, supplied to the inner closure for the back-edge. A user should never be able to construct this.
    Unowned(Weak<P>),
}
impl<P: ?Sized> RecursiveInner<P> {
    fn as_ptr(&self) -> *const P {
        match self {
            RecursiveInner::Owned(owned) => owned as &P as *const P,
            RecursiveInner::Unowned(weak) => weak.as_ptr(),
        }
    }
    fn recursive_id(&self) -> RecursiveId {
        RecursiveId(self.as_ptr() as *const ())
    }
}

// Regarding the lint: constraining I and E without these is impossible,
// and in order to use this type alias to begin with, you need to fulfill them.
/// Type alias of [`dyn Parser<'src, I, A, E>`](Parser) for use in [`Recursive`].
#[allow(type_alias_bounds)]
pub type Direct<'src, I: Input + ?Sized, A, E: ParseEnvironment> = dyn Parser<'src, I, A, E> + 'src;

/// The inner type for mutually-recursive combinators formed via [`Recursive::declare`] and [`Recursive::define`].
pub struct Indirect<'src, I: Input + ?Sized, A, E: ParseEnvironment> {
    description: Option<Cow<'static, str>>,
    inner: OnceCell<Box<dyn Parser<'src, I, A, E> + 'src>>,
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P: ?Sized> Parser<'src, I, A, E> for Recursive<P>
where
    P : Parser<'src, I, A, E>,
    A : Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>, ParseOutput<A, E>) {
        // # Internal Notes
        //
        // This function contains the workhouse of the recursive seed-growing algorithm,
        // but it delegates to the inner `P` for parsing,
        // hence `Parser` must be implemented for `RecursiveInner`, `Direct` and `Indirect`,
        // though these are (mostly) straightforward wrapper types.

        match state.recursive_stack.get_latest(self.recursive_id(), state.cursor) {

            Some(frame) => {
                // encountered. check for seed?
                match &frame.seed_result {

                    // clone seed value, restore its type and return.
                    Some((seed_state, erased)) => {
                        assert_eq!(
                            frame.id,
                            self.recursive_id(),
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
                                description: self.description.as_ref()
                                    .map(|s| s.clone().into_owned()),
                                diagnostic_name: self.diagnostic_name(),
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
                // first entry; seed-growing.
                let mut recursive_stack
                    = state.recursive_stack.push_to_top(RecursiveFrame {
                        id: self.recursive_id(),
                        start_cursor: state.cursor,
                        description: self.description.clone(),
                        diagnostic_name: self.diagnostic_name(),
                        seed_result: None,
                    });
                let seed_state = ParseState {
                    recursive_stack: recursive_stack.clone(),
                    ..state.clone()
                };

                let parser = &self.inner;

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
                        id: self.recursive_id(),
                        start_cursor: state.cursor,
                        description: self.description.clone(),
                        diagnostic_name: self.diagnostic_name(),
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

// The helper implementors:

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P: ?Sized> Parser<'src, I, A, E> for RecursiveInner<P>
where
    P : Parser<'src, I, A, E>,
    A : Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>, ParseOutput<A, E>)
    {
        match self {
            RecursiveInner::Owned(owned) => {
                owned.as_ref().parse(input, state)
            },
            RecursiveInner::Unowned(weak) => {
                weak.upgrade()
                    .expect("Weak pointer is unconstructible by users; source must be alive!")
                    .parse(input, state)
            },
        }
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>, ParseOutput<(), E>)
    {
        match self {
            RecursiveInner::Owned(owned) => {
                owned.as_ref().check(input, state)
            },
            RecursiveInner::Unowned(weak) => {
                weak.upgrade()
                    .expect("Weak pointer is unconstructible by users; source must be alive!")
                    .check(input, state)
            },
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment> Parser<'src, I, A, E> for Indirect<'src, I, A, E>
where
    A : Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>, ParseOutput<A, E>)
    {
        self.inner.get()
            .unwrap_or_else(|| panic!("Called Parser::parse on undefined combinator{}!",
                    self.description.as_ref().map(|s| format!(" '{s}'")).unwrap_or("".into())))
            .parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>, ParseOutput<(), E>)
    {
        self.inner.get()
            .unwrap_or_else(|| panic!("Called Parser::parse on undefined combinator{}!",
                    self.description.as_ref().map(|s| format!(" '{s}'")).unwrap_or("".into())))
            .check(input, state)
    }
}