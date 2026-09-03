use std::{
    borrow::Cow,
    cell::OnceCell,
    fmt::Debug,
    rc::{Rc, Weak},
};

use crate::{
    combinator::{
        RecursiveState,
        recursive::frame::{ErasedOutput, RecursiveFrame, RecursiveId},
    },
    cst::CstOutput,
    failure::{FailureKind, ParseFailure},
    input::Input,
    parse::{ParseEnvironment, ParseHint, ParseOutput, ParseResult, ParseState, Parser},
};

// The following structures are basically copied from `chumsky`'s approach to their `Recursive` combinator.

#[derive(Debug)]
pub struct Recursive<P: ?Sized> {
    /// Inner payload.
    inner: RecursiveInner<P>,
    /// Description.
    description: Option<Cow<'static, str>>,
}
impl<P: ?Sized> Clone for Recursive<P> {
    fn clone(&self) -> Self {
        Recursive {
            inner: self.inner.clone(),
            description: self.description.clone(),
        }
    }
}
impl<P: ?Sized> Recursive<P> {
    fn recursive_id(&self) -> RecursiveId {
        self.inner.recursive_id()
    }
    fn diagnostic_name(&self) -> String {
        format!("<{:p}>", self.inner.as_ptr() as *const ())
    }
    fn name(&self) -> String {
        self.description
            .clone()
            .unwrap_or_else(|| self.diagnostic_name().into())
            .into_owned()
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment> Recursive<Direct<'src, I, A, E>> {
    /// Creates a new self-recursive combinator.
    pub fn new<'p, F, P>(closure: F) -> Self
    where
        F: FnOnce(Recursive<Direct<'src, I, A, E>>) -> P,
        P: Parser<'src, I, A, E> + 'p,
        'p: 'src,
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

    /// Creates a new self-recursive combinator with the given name.
    /// Combinator names are used for error reporting and better diagnostics.
    pub fn new_as<'p, F, P>(s: impl Into<Cow<'static, str>>, closure: F) -> Self
    where
        F: FnOnce(Recursive<Direct<'src, I, A, E>>) -> P,
        P: Parser<'src, I, A, E> + 'p,
        'p: 'src,
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
    /// Declares a new recursive combinator.
    ///
    /// This is preferred when direct self-recursion is insufficient or unergonomic,
    /// and a named object is required to define mutual recursion between related combinators.
    pub fn declare() -> Self {
        Recursive {
            inner: RecursiveInner::Owned(Rc::new(Indirect {
                description: None,
                inner: OnceCell::new(),
            })),
            description: None,
        }
    }
    /// Declares a new recursive combinator with the given name.
    /// Combinator names are used for error reporting and better diagnostics.
    ///
    /// This is preferred when direct self-recursion is insufficient or unergonomic,
    /// and a named object is required to define mutual recursion between related combinators.
    pub fn declare_as(s: impl Into<Cow<'static, str>>) -> Self {
        Recursive {
            inner: RecursiveInner::Owned(Rc::new(Indirect {
                description: Some(s.into()),
                inner: OnceCell::new(),
            })),
            description: None,
        }
    }
    /// Defines a parser to use the following inner definition.
    ///
    /// # Limitation on Strong Cycles
    ///
    /// Strong reference cycles formed by [`Recursive<Indirect<...>>`] groups (including `self` cycles)
    /// *never* deallocate, even after becoming inaccessible, throughout the lifetime of the program.
    ///
    /// This is currently considered acceptable because parsers typically are constructed no more than once,
    /// and stored and reused throughout the lifetime of the program.
    /// In other words, the memory leak is bounded to the creation of a mutually-recursive combinator group.
    ///
    /// Future mitigation of this would entail a `RecursiveGroup`-like structure involving n-variadic type arguments.
    pub fn define<'p, P>(&mut self, parser: P)
    where
        P: Parser<'src, I, A, E> + 'p,
        'p: 'src,
    {
        match &mut self.inner {
            RecursiveInner::Owned(owned) => owned
                .as_ref()
                .inner
                .set(Box::new(parser))
                .unwrap_or_else(|_| {
                    panic!(
                        "Called Recursive::define on already-defined combinator{}!",
                        self.description
                            .as_ref()
                            .map(|s| format!(" '{s}'"))
                            .unwrap_or("".into())
                    )
                }),
            RecursiveInner::Unowned(_) => {
                panic!(
                    "Called Recursive::define on weak reference{}!",
                    self.description
                        .as_ref()
                        .map(|s| format!(" '{s}'"))
                        .unwrap_or("".into())
                )
            }
        }
    }
}

#[derive(Debug)]
enum RecursiveInner<P: ?Sized> {
    /// Owned. What users get during direct construction, declaration and definition.
    Owned(Rc<P>),
    /// Unowned, supplied to the inner closure for the back-edge of self-recursion.
    /// A user should never be able to construct this.
    Unowned(Weak<P>),
}
impl<P: ?Sized> Clone for RecursiveInner<P> {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(owned) => Self::Owned(owned.clone()),
            Self::Unowned(weak) => Self::Unowned(weak.clone()),
        }
    }
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

/// # Internal Notes
///
/// - This function contains the workhouse of the recursive seed-growing algorithm,
///   but it delegates to the inner `P` for parsing,
///   hence `Parser` must be implemented for `RecursiveInner`, `Direct` and `Indirect`,
///   though these are (mostly) straightforward wrapper types.
/// - This has been separated into a macro because we need to dispatch on `parser.parse(...)` and `parser.check(...)`,
///   which have differing signatures and different names, thus not abstractible as a method.
///   You should conceptually think of this as a method of `Recursive` though.
macro_rules! recursive_growth {
    ($self:expr, $input:expr, $state:expr, $method:ident, $payload:tt) => {{
        // Convenience let-bindings.
        // A shame we can't really do this for self.
        let this = ($self);
        let input = ($input);
        let state = ($state);

        // println!("ENTER: {}@{:?}", &this.name(), state.cursor);

        // Check the recursive state on whether or not this recursive function has been entered.
        match state.recursive_state.get(this.recursive_id(), state.cursor) {

            Some(frame) => {
                // check for stale-ness. This is only subject to non-active fixpoints.
                if !frame.active_fixpoint && this.entry_is_stale_or_absent(state.cursor, &state.recursive_state) {
                    // let Some(frame) = state.recursive_state.get_mut(this.recursive_id(), state.cursor)
                    // else { unreachable!() };
                    // frame.dependencies = im::Vector::new();
                    // println!("CACHE STALE!: {}@{:?}", &this.name(), state.cursor);
                    return recompute!(this, input, state, $method);
                }

                // encountered. check for seed?
                match &frame.seed_result {

                    // seed found.
                    // clone seed value, restore its type and return.
                    Some((seed_state, erased)) => {
                        assert_eq!(
                            typeid::of::<ParseOutput<$payload, E>>(),
                            erased.type_id(),
                            "recursive output has different expected type!"
                        );

                        // println!("CACHED SEED: {}@{:?}", &this.name(), state.cursor);

                        // # SAFETY:
                        // 1. Every `RecursiveFrame` is associated with exactly one `RecursiveId`,
                        //    identifying the Recursive combinator whose invocation the frame represents.
                        // 2. Each `RecursiveId` uniquely corresponds to exactly one output type in this invocation,
                        //    which is identified by `erased.type_id()`.
                        // 3. Following 1 and 2: the erased payload is asserted to be
                        //    created by this Recursive combinator in this invocation,
                        //    hence the output must be of type:
                        //    - `ParseOutput<A, E>` for `parse`;
                        //    - `ParseOutput<(), E>` for `check`.
                        // 4. `ErasedOutput::as_ptr` points directly to its contained `ParseOutput<_, E>`.
                        //    We borrow that value and clone it; ownership of the erased output remains unchanged.
                        // 5. Lifetime `'src` of A is recovered during cloning from the recursive state.
                        //    A non-`'src` payload will not survive the parser invocation from which it originates.
                        //    Hence, using `::typeid::of` is sufficient to establish type identity despite lifetime erasure.
                        let parse_output = unsafe {
                            let output = &*(erased.as_ptr() as *const ParseOutput<$payload, E>);
                            output.clone()
                        };

                        // Overwrite seed state's recursive state with that discovered from the current state.
                        // Hence, update dependency to the enclosure.
                        let mut return_state = seed_state.clone();
                        return_state.recursive_state = state.recursive_state.clone();
                        return_state.recursive_state.current_dependencies
                            .push_back((this.recursive_id(), state.cursor, frame.generation));
                        (return_state, parse_output)
                    }

                    None => {
                        // println!("NO SEED: {}@{:?}", &this.name(), state.cursor);

                        // speculative error for left-recursion.
                        // no need to record failure dependency, because either:
                        // - speculative failure: replaced by alternative, or propagated.
                        // - committed failure: propagated, seed value not used.
                        let failure = E::Failure::failure_at::<_, E::Failure>(
                            input,
                            state.cursor,
                            FailureKind::ZeroProgressRecursion {
                                name: this.description.as_ref().map(|s| s.clone().into_owned())
                            },
                        );

                        // Overwrite seed state's recursive state with that discovered from the current state.
                        let return_state = state.clone();
                        (
                            return_state,
                            ParseOutput {
                                ast: ParseResult::Speculative(failure),
                                cst: CstOutput::new(),
                            },
                        )
                    }
                }
            }

            None => {
                // println!("FIRST ENTRY!: {}@{:?}", &this.name(), state.cursor);
                recompute!(this, input, state, $method)
            }
        }
    }};
}
macro_rules! recompute {
    ($self:expr, $input:expr, $state:expr, $method:ident) => {{
        // Convenience let-bindings.
        // A shame we can't really do this for self.
        let this = ($self);
        let input = ($input);
        let state = ($state);

        // first entry; seed-growing algorithm.
        let outer_enclosure = state.recursive_state.current_enclosing.clone();
        let outer_dependencies = state.recursive_state.current_dependencies.clone();

        // # SAFETY
        //
        // Invariant regarding `RecursiveFrame` construction fulfilled;
        // This is the only location where it is constructed, and subsequent modifications do not change `id`.
        let mut recursive_state = state.recursive_state.clone();
        recursive_state.push_to_top(this.recursive_id(), state.cursor,
            RecursiveFrame {
                description: this.description.clone(),
                diagnostic_name: this.diagnostic_name(),
                seed_result: None,
                active_fixpoint: true,
                dependencies: im::Vector::new(),
                generation: 0,
            });
        let seed_state = ParseState {
            recursive_state,
            ..state.clone()
        };

        let parser = &this.inner;

        // Check if the initial sprout encountered failures.
        // If yes, there is no point in seed growing, and the failure should be propagated.
        let (mut sprout_state, sprout_result) = parser.$method(input, seed_state);
        match &sprout_result.ast {
            ParseResult::Speculative(_) | ParseResult::Committed(_) => {
                // failure to sprout.
                sprout_state.recursive_state.current_enclosing = outer_enclosure;
                sprout_state.recursive_state.current_dependencies = outer_dependencies;
                return (sprout_state, sprout_result);
            }
            // Succcessful seed.
            // Any speculative errors now would be an indication to halt.
            // Committed errors should still be propagated though.
            ParseResult::Success(_) => (),
        }

        let (mut rec_seed_state, mut rec_seed_result) = (sprout_state, sprout_result);

        let (mut fixpoint_state, fixpoint_result) = loop {
            // Entering from previous success:
            // Modify the seed result, dependencies, and add to generation.
            let mut frame = rec_seed_state.recursive_state
                .get_mut(this.recursive_id(), state.cursor).cloned()
                .expect("Frame must have been allocated from above!");
            frame.seed_result = Some((rec_seed_state.clone(), rec_seed_result.clone_erased()));
            frame.dependencies = rec_seed_state.recursive_state.current_dependencies.clone();
            frame.generation += 1;
            // println!("GENERATION {}: {}@{:?}", frame.generation, &this.name(), state.cursor);
            rec_seed_state.recursive_state.push_to_top(this.recursive_id(), state.cursor, frame);

            // Subsequent iteration
            let current_best_cursor = rec_seed_state.cursor;
            let current_state = ParseState {
                recursive_state: rec_seed_state.recursive_state.clone(),
                ..state.clone()
            };
            let (next_sprout_state, next_sprout_result) =
                parser.$method(input, current_state);

            match &next_sprout_result.ast {
                ParseResult::Success(_) => {
                    // Grow the seed and continue, if not zero-progress.
                    // println!("Best: {:?}\nCandidate: {:?}", current_best_cursor, next_sprout_state.cursor);
                    if next_sprout_state.cursor > current_best_cursor {
                        // Assign to the recursive seed state.
                        rec_seed_state = next_sprout_state;
                        rec_seed_result = next_sprout_result;
                        continue;
                    } else {
                        // Zero (or negative) progress encountered.
                        // Treat stagnancy similarly to speculative error:
                        // reached fixpoint and break.
                        break (rec_seed_state, rec_seed_result);
                    }
                }
                ParseResult::Speculative(_) => {
                    // The previous iteration has consumed as much as we can without speculative failure.
                    // It's time to stop.
                    break (rec_seed_state, rec_seed_result);
                }
                ParseResult::Committed(_) => {
                    // Propagate failure.
                    break (next_sprout_state, next_sprout_result);
                }
            }
        };

        // Get frame and disable activity.
        let frame = fixpoint_state.recursive_state
            .get_mut(this.recursive_id(), state.cursor)
            .expect("Frame must have been allocated from above!");
        frame.active_fixpoint = false;
        let generation = frame.generation;
        // Restore to outer enclosing.
        fixpoint_state.recursive_state.current_enclosing = outer_enclosure;
        fixpoint_state.recursive_state.current_dependencies
            .push_back((this.recursive_id(), state.cursor, generation));

        (fixpoint_state, fixpoint_result)
    }};
}
impl<P: ?Sized> Recursive<P> {
    /// Invariant: self at this cursor must not be actively participating in fixpoint calculation.
    fn entry_is_stale_or_absent<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
        &self,
        cursor: I::Cursor,
        state: &RecursiveState<'src, I::Cursor, E::Supply>,
    ) -> bool
    where
        P: Parser<'src, I, A, E>,
        A: Clone + 'src,
    {
        let Some(frame) = state.get(self.recursive_id(), cursor) else {
            return true;
        };
        frame
            .dependencies
            .iter()
            .any(|(dep_id, dep_cursor, dep_gen)| {
                if *dep_id == self.recursive_id() && *dep_cursor == cursor {
                    return false;
                }
                state
                    .get(*dep_id, *dep_cursor)
                    .is_none_or(|current| current.generation != *dep_gen)
            })
    }
}

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P: ?Sized> Parser<'src, I, A, E>
    for Recursive<P>
where
    P: Parser<'src, I, A, E>,
    A: Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<A, E>,
    ) {
        recursive_growth!(self, input, state, parse, A)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        recursive_growth!(self, input, state, check, ())
    }
    fn hint(&self) -> ParseHint {
        if let Some(inner) = &self.description {
            ParseHint::Named {
                name: inner.clone().into_owned(),
                inner: Box::new(ParseHint::Opaque),
            }
        } else {
            ParseHint::Opaque
        }
    }
}

// The helper implementors:

impl<'src, I: Input + ?Sized, A, E: ParseEnvironment, P: ?Sized> Parser<'src, I, A, E>
    for RecursiveInner<P>
where
    P: Parser<'src, I, A, E>,
    A: Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<A, E>,
    ) {
        match self {
            RecursiveInner::Owned(owned) => owned.as_ref().parse(input, state),
            RecursiveInner::Unowned(weak) => weak
                .upgrade()
                .expect("Weak pointer is unconstructible by users; source must be alive!")
                .parse(input, state),
        }
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        match self {
            RecursiveInner::Owned(owned) => owned.as_ref().check(input, state),
            RecursiveInner::Unowned(weak) => weak
                .upgrade()
                .expect("Weak pointer is unconstructible by users; source must be alive!")
                .check(input, state),
        }
    }
}
impl<'src, I: Input + ?Sized, A, E: ParseEnvironment> Parser<'src, I, A, E>
    for Indirect<'src, I, A, E>
where
    A: Clone + 'src,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<A, E>,
    ) {
        self.inner
            .get()
            .unwrap_or_else(|| {
                panic!(
                    "Called Parser::parse on undefined combinator{}!",
                    self.description
                        .as_ref()
                        .map(|s| format!(" '{s}'"))
                        .unwrap_or("".into())
                )
            })
            .parse(input, state)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
    ) -> (
        ParseState<'src, <I as Input>::Cursor, <E as ParseEnvironment>::Supply>,
        ParseOutput<(), E>,
    ) {
        self.inner
            .get()
            .unwrap_or_else(|| {
                panic!(
                    "Called Parser::parse on undefined combinator{}!",
                    self.description
                        .as_ref()
                        .map(|s| format!(" '{s}'"))
                        .unwrap_or("".into())
                )
            })
            .check(input, state)
    }
}
