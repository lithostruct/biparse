use std::{
    borrow::Cow,
    cell::OnceCell,
    fmt::Debug,
    rc::{Rc, Weak},
};

use crate::{
    combinator::{
        recursive::frame::RecursiveState,
        recursive::frame::{ErasedOutput, RecursiveFrame, RecursiveId},
    },
    cst::AssociatedGreenCst,
    failure::{FailureKind, ParseFailure, UnboundedRecursion},
    input::Input,
    parse::{
        ParseEnvironment, ParseHint, ParseOutcome, ParseOutput, ParseState, ParseStateData, Parser,
    },
};

use arcstr::ArcStr;

// The following structures are basically copied from `chumsky`'s approach to their `Recursive` combinator.

#[derive(Debug)]
pub struct Recursive<P: ?Sized> {
    /// Inner payload.
    inner: RecursiveInner<P>,
}
impl<P: ?Sized> Clone for Recursive<P> {
    fn clone(&self) -> Self {
        Recursive {
            inner: self.inner.clone(),
        }
    }
}
impl<P: ?Sized> Recursive<P> {
    #[inline(always)]
    fn recursive_id(&self) -> RecursiveId {
        self.inner.recursive_id()
    }
    #[inline(always)]
    fn diagnostic_name(&self) -> String {
        format!("<{:p}>", self.inner.as_ptr() as *const ())
    }

    #[inline(always)]
    fn name(&self) -> Option<ArcStr> {
        self.inner.name()
    }
    #[inline(always)]
    fn name_cow(&self) -> Option<Cow<'static, str>> {
        self.inner.name_cow()
    }

    fn given_or_diagnostic_name(&self) -> String {
        self.name()
            .as_ref()
            .map_or_else(|| self.diagnostic_name(), |inner| inner.to_string())
    }
}
impl<'p, 'src, I: Input + ?Sized, A, E: ParseEnvironment> Recursive<Direct<'p, 'src, I, A, E>> {
    /// Creates a new self-recursive combinator.
    /// The closure taken will be provided a reference to the created combinator (`self`).
    pub fn new<F, P>(closure: F) -> Self
    where
        F: FnOnce(Self) -> P,
        P: Parser<'src, I, A, E> + 'p,
    {
        let rc: Rc<P> = Rc::new_cyclic(|weak| {
            let this: Recursive<Direct<'p, 'src, I, A, E>> = Recursive {
                inner: RecursiveInner::Unowned(
                    None,
                    weak.clone() as Weak<Direct<'p, 'src, I, A, E>>,
                ),
            };
            closure(this)
        });
        Recursive {
            inner: RecursiveInner::Owned(None, rc as Rc<Direct<'p, 'src, I, A, E>>),
        }
    }

    /// Creates a new self-recursive combinator with the given name.
    /// Combinator names are used for error reporting and better diagnostics.
    /// The closure taken will be provided a reference to the created combinator (`self`).
    pub fn new_as<F, P>(name: impl Into<Cow<'static, str>>, closure: F) -> Self
    where
        F: FnOnce(Self) -> P,
        P: Parser<'src, I, A, E> + 'p,
    {
        let name: Option<ArcStr> = Some(name.into().into());
        let rc: Rc<P> = Rc::new_cyclic(|weak| {
            let this: Recursive<Direct<'p, 'src, I, A, E>> = Recursive {
                inner: RecursiveInner::Unowned(
                    name.clone(),
                    weak.clone() as Weak<Direct<'p, 'src, I, A, E>>,
                ),
            };
            closure(this)
        });
        Recursive {
            inner: RecursiveInner::Owned(name, rc as Rc<Direct<'p, 'src, I, A, E>>),
        }
    }
}

impl<'p, 'src, I: Input + ?Sized, A, E: ParseEnvironment> Recursive<Indirect<'p, 'src, I, A, E>> {
    /// Declares a new recursive combinator to be later defined via [`Recursive::define`].
    ///
    /// This is preferred when direct self-recursion is insufficient or unergonomic,
    /// and a named object is required to define mutual recursion between related combinators.
    ///
    /// Attempting a parse or check with a declared but undefined recursive parser
    /// immediately results in a [`ParseOutcome::Fatal`] failure.
    pub fn declare() -> Self {
        Recursive {
            inner: RecursiveInner::Owned(
                None,
                Rc::new(Indirect {
                    name: None,
                    inner: OnceCell::new(),
                }),
            ),
        }
    }
    /// Declares a new recursive combinator to be later defined via [`Recursive::define`], with the given name.
    /// Combinator names are used for error reporting and better diagnostics.
    ///
    /// This is preferred when direct self-recursion is insufficient or unergonomic,
    /// and a named object is required to define mutual recursion between related combinators.
    ///
    /// Attempting a parse or check with a declared but undefined recursive parser
    /// immediately results in a [`ParseOutcome::Fatal`] failure.
    pub fn declare_as(name: impl Into<Cow<'static, str>>) -> Self {
        let name: Option<ArcStr> = Some(name.into().into());
        Recursive {
            inner: RecursiveInner::Owned(
                name.clone(),
                Rc::new(Indirect {
                    name: name.clone(),
                    inner: OnceCell::new(),
                }),
            ),
        }
    }

    /// Defines a parser to use the following inner definition.
    /// The parser object must be strongly-owned.
    ///
    /// # Panics
    ///
    /// Panics on multiple or re-entrant definition of the same parser object,
    /// or attempting to define a weak handle as produced by [`Recursive::weaken`].
    ///
    /// # Limitation on Strong Cycles
    ///
    /// Strong reference cycles formed by [`Recursive<Indirect<...>>`] groups (including `self` cycles)
    /// *never* deallocate, even after becoming inaccessible, throughout the lifetime of the program.
    ///
    /// Use [`Recursive::weaken`] where applicable to break recursive back-edges.
    ///
    /// This may be addressed more completely in the future with explicit group-like structure
    /// involving either n-variadic type arguments or payload type erasure.
    pub fn define<P>(&mut self, parser: P)
    where
        P: Parser<'src, I, A, E> + 'p,
    {
        match &mut self.inner {
            RecursiveInner::Owned(_, owned) => owned
                .as_ref()
                .inner
                .set(Box::new(parser))
                .unwrap_or_else(|_| {
                    panic!(
                        "Called Recursive::define on already-defined combinator{}!",
                        self.name().map(|s| format!(" '{s}'")).unwrap_or("".into())
                    )
                }),
            RecursiveInner::Unowned(..) => {
                panic!(
                    "Called Recursive::define on weak reference{}!",
                    self.name().map(|s| format!(" '{s}'")).unwrap_or("".into())
                )
            }
        }
    }

    /// Produces a non-owning handle to `self`.
    /// This is primarily used to break strong recursive cycles,
    /// allowing for mutually-recursive parsers to be deallocated.
    ///
    /// # Note to Users
    ///
    /// A handle produced by this function will not stop the underlying allocation
    /// from being deallocated when all owning handles are dropped.
    /// Attempting a parse on a dropped parser results in a [`ParseOutcome::Fatal`] parse failure.
    ///
    /// This typically entails that there is one "primary" parser,
    /// which keeps its group of mutually-recursive combinators alive.
    /// [`Recursive::weaken`] need only be invoked for recursive back edges to this primary parser
    /// to ensure the group may be deallocated.
    pub fn weaken(&self) -> Self {
        Recursive {
            inner: RecursiveInner::Unowned(self.inner.name(), self.inner.as_weak()),
        }
    }
}

/// # Note on First Field: Why `ArcStr`?
///
/// Recursive constructs are cloned a lot. This makes that cheaper.
/// Since the name is an important field supplied as `Cow<'static, str>`,
/// where the literal string is the (extremely) common case,
/// `ArcStr` can be zero-copy and no-atomic.
///
/// This is to be converted at API boundaries.
#[derive(Debug)]
enum RecursiveInner<P: ?Sized> {
    /// Owned. What users get during direct construction, declaration and definition.
    Owned(Option<ArcStr>, Rc<P>),
    /// Unowned, supplied to the inner closure for the back-edge of self-recursion.
    /// A user may construct this via [`Recursive::weaken`].
    Unowned(Option<ArcStr>, Weak<P>),
}
impl<P: ?Sized> Clone for RecursiveInner<P> {
    fn clone(&self) -> Self {
        match self {
            Self::Owned(name, owned) => Self::Owned(name.clone(), owned.clone()),
            Self::Unowned(name, weak) => Self::Unowned(name.clone(), weak.clone()),
        }
    }
}
impl<P: ?Sized> RecursiveInner<P> {
    fn as_ptr(&self) -> *const P {
        match self {
            RecursiveInner::Owned(_, owned) => owned as &P as *const P,
            RecursiveInner::Unowned(_, weak) => weak.as_ptr(),
        }
    }
    fn recursive_id(&self) -> RecursiveId {
        RecursiveId(self.as_ptr() as *const ())
    }

    fn as_weak(&self) -> Weak<P> {
        match self {
            RecursiveInner::Owned(_, rc) => Rc::downgrade(rc),
            RecursiveInner::Unowned(_, weak) => weak.clone(),
        }
    }
    // fn as_rc(&self) -> Option<Rc<P>> {
    //     match self {
    //         RecursiveInner::Owned(_, rc) => Some(rc.clone()),
    //         RecursiveInner::Unowned(_, weak) => weak.upgrade(),
    //     }
    // }

    fn name(&self) -> Option<ArcStr> {
        match self {
            RecursiveInner::Owned(name, _) | RecursiveInner::Unowned(name, _) => name.clone(),
        }
    }
    fn name_cow(&self) -> Option<Cow<'static, str>> {
        match self {
            RecursiveInner::Owned(Some(named), _) | RecursiveInner::Unowned(Some(named), _) => {
                Some(
                    ArcStr::as_static(named)
                        .map_or_else(|| Cow::Owned(named.to_string()), Cow::Borrowed),
                )
            }
            _ => None,
        }
    }
}

// Regarding the lint: constraining I and E without these is impossible,
// and in order to use this type alias to begin with, you need to fulfill them.
/// Type alias of [`dyn Parser<'src, I, A, E>`](Parser) for use in [`Recursive`].
#[allow(type_alias_bounds)]
pub type Direct<'p, 'src, I: Input + ?Sized, A, E: ParseEnvironment> =
    dyn Parser<'src, I, A, E> + 'p;

/// The inner type for mutually-recursive combinators formed via [`Recursive::declare`] and [`Recursive::define`].
pub struct Indirect<'p, 'src, I: Input + ?Sized, A, E: ParseEnvironment> {
    name: Option<ArcStr>,
    inner: OnceCell<Box<dyn Parser<'src, I, A, E> + 'p>>,
}

/// Debug log enabler. Writes to stderr.
const PRINT_DEBUG_LOGS: bool = false;
macro_rules! debug_log {
    ($($arg:expr),*) => {
        if PRINT_DEBUG_LOGS {
            eprintln!($($arg,)*)
        }
    };
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

        debug_log!("ENTER: {} @ {:?}", &this.given_or_diagnostic_name(), input.byte_offset(state.cursor));

        // Check the recursive state on whether or not this recursive function has been entered.
        match state.recursive_state.get(this.recursive_id(), state.cursor) {

            Some(frame) => {
                // Check for stale-ness. This is only subject to non-active fixpoints.
                if !frame.active_fixpoint && this.entry_is_stale_or_absent(state.cursor, &state.recursive_state) {
                    return recompute!(this, input, state, $method);
                }

                // Encountered. check for seed?
                match &frame.seed_result {

                    // Seed found.
                    // Clone seed value, restore its type and return.
                    Some((seed_state, erased)) => {
                        assert_eq!(
                            std::any::TypeId::of::<ParseOutput<$payload, E>>(),
                            erased.type_id(),
                            "recursive output has different expected type!"
                        );

                        debug_log!("CACHED SEED: {} @ {:?} -> {:?}",
                            &this.given_or_diagnostic_name(),
                            input.byte_offset(state.cursor),
                            input.byte_offset(seed_state.cursor)
                        );

                        // SAFETY:
                        // 1. Every `RecursiveFrame` is associated with exactly one `RecursiveId`,
                        //    identifying the Recursive combinator whose invocation the frame represents.
                        // 2. Each `RecursiveId` uniquely corresponds to exactly one output type in this invocation,
                        //    which is asserted by `erased.type_id()`.
                        // 3. Following 1 and 2: the erased payload is asserted to be
                        //    created by this Recursive combinator in this invocation,
                        //    hence the output must be of type:
                        //    - `ParseOutput<A, E>` for `parse`;
                        //    - `ParseOutput<(), E>` for `check`.
                        // 4. `ErasedOutput::as_ptr` points directly to `ParseOutput<_, E>`.
                        //    We borrow that value and clone it;
                        //    ownership of the erased output remains refcounted within RecursiveState instances.
                        // 5. Lifetime of A, `E::Failure` and the CST data are of lifetime `'static`,
                        //    hence `std::any::TypeId` is sufficient to establish type equality,
                        //    guarding against unsound type downcasting.
                        let parse_output = unsafe {
                            let output = &*(erased.as_ptr() as *const ParseOutput<$payload, E>);
                            output.clone()
                        };

                        // Overwrite seed state's recursive state with that discovered from the current state,
                        // therefore preserving any recursive evaluation progress made after the seed is cached.
                        // Hence, update dependency to the enclosure.
                        let mut return_state = seed_state.clone();
                        return_state.recursive_state = state.recursive_state.clone();
                        return_state.recursive_state.current_dependencies
                            .push_back((this.recursive_id(), state.cursor, frame.generation));
                        (return_state, parse_output)
                    }

                    None => {
                        debug_log!("NO SEED: {} @ {:?}", &this.given_or_diagnostic_name(), input.byte_offset(state.cursor));

                        // Speculative error for left-recursion.
                        // No need to record failure dependency, because either:
                        // - speculative failure: replaced by alternative, or propagated.
                        // - committed failure: propagated, seed value not used.
                        let failure = E::Failure::unbounded_recursion(
                            input,
                            state.cursor,
                            UnboundedRecursion {
                                name: this.name_cow(),
                                from: this.recursive_id()
                            }
                        );

                        // Overwrite seed state's recursive state with that discovered from the current state.
                        let return_state = state.clone();
                        (
                            return_state,
                            ParseOutput {
                                ast: ParseOutcome::Speculative(failure),
                                cst: AssociatedGreenCst::new(),
                            },
                        )
                    }
                }
            }

            None => {
                debug_log!("FIRST ENTRY!: {} @ {:?}", &this.given_or_diagnostic_name(), input.byte_offset(state.cursor));
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
        // This is the only location where it is constructed.
        let mut recursive_state = state.recursive_state.clone();
        recursive_state.push_to_top(this.recursive_id(), state.cursor,
            RecursiveFrame {
                description: this.name(),
                diagnostic_name: this.diagnostic_name(),
                allocation_store: Box::new(this.inner.as_weak()),
                seed_result: None,
                active_fixpoint: false,
                dependencies: im::Vector::new(),
                generation: 0,
            });
        let seed_state = ParseStateData {
            recursive_state,
            ..state.clone()
        };

        let parser = &this.inner;

        // Check if the initial sprout encountered failures.
        // If yes, there is no point in seed growing, and the failure should be propagated.
        let (mut sprout_state, mut sprout_output) = parser.$method(input, seed_state);
        match &sprout_output.ast {
            ParseOutcome::Speculative(failure)
            | ParseOutcome::Committed(failure) => {
                // failure to sprout.
                sprout_state.recursive_state.current_enclosing = outer_enclosure;
                sprout_state.recursive_state.current_dependencies = outer_dependencies;

                // check if the failure is due to unbounded recursion. If yes, promote.
                if failure.is_unbounded_recursion_at(input, state.cursor, UnboundedRecursion {
                    name: this.name_cow(),
                    from: this.recursive_id()
                }) {
                    // Unbounded recursion identified as primary failure mode.
                    sprout_output.ast = ParseOutcome::Fatal(failure.clone());
                }

                return (sprout_state, sprout_output);
            }
            ParseOutcome::Fatal(_) => {
                // fatal failure to sprout.
                sprout_state.recursive_state.current_enclosing = outer_enclosure;
                sprout_state.recursive_state.current_dependencies = outer_dependencies;
                return (sprout_state, sprout_output);
            }

            // Successful seed.
            // Any speculative errors now would be an indication to halt.
            // Committed errors should still be propagated though.
            ParseOutcome::Success(_) => (),
        }

        let (mut rec_seed_state, mut rec_seed_output) = (sprout_state, sprout_output);

        let (mut fixpoint_state, fixpoint_output) = loop {
            // Entering from previous success:
            // Modify the seed result, dependencies, and add to generation.
            let mut frame = rec_seed_state.recursive_state
                .get_mut(this.recursive_id(), state.cursor).cloned()
                .expect("Frame must have been allocated from above!");
            frame.active_fixpoint = true;
            frame.seed_result = Some((rec_seed_state.clone(), rec_seed_output.clone_erased()));
            frame.dependencies = rec_seed_state.recursive_state.current_dependencies.clone();
            frame.generation += 1;

            debug_log!("GENERATION {}: {} @ {:?}", frame.generation, &this.given_or_diagnostic_name(), input.byte_offset(state.cursor));
            rec_seed_state.recursive_state.push_to_top(this.recursive_id(), state.cursor, frame);

            // Subsequent iteration.
            let current_best_cursor = rec_seed_state.cursor;
            let current_state = ParseStateData {
                recursive_state: rec_seed_state.recursive_state.clone(),
                ..state.clone()
            };
            let (next_sprout_state, next_sprout_result) =
                parser.$method(input, current_state);

            match &next_sprout_result.ast {
                ParseOutcome::Success(_) => {
                    // Grow the seed and continue, if not zero-progress.
                    debug_log!("{} @ {:?}: Best: {:?}; Candidate: {:?}",
                        &this.given_or_diagnostic_name(),
                        input.byte_offset(state.cursor),
                        input.byte_offset(current_best_cursor),
                        input.byte_offset(next_sprout_state.cursor)
                    );

                    if next_sprout_state.cursor > current_best_cursor {
                        // Assign to the recursive seed state.
                        rec_seed_state = next_sprout_state;
                        rec_seed_output = next_sprout_result;
                        continue;
                    } else {
                        // Zero (or negative) progress encountered.
                        // Treat stagnancy similarly to speculative error:
                        // reached fixpoint and break.
                        //
                        // This returns the previous result prior to encountering zero/negative progress.
                        break (rec_seed_state, rec_seed_output);
                    }
                }
                ParseOutcome::Speculative(_) => {
                    // The previous iteration has consumed as much as we can without speculative failure.
                    // It's time to stop.
                    break (rec_seed_state, rec_seed_output);
                }
                ParseOutcome::Committed(_) | ParseOutcome::Fatal(_) => {
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

        (fixpoint_state, fixpoint_output)
    }};
}
impl<P: ?Sized> Recursive<P> {
    /// Invariant: self at this cursor must not be actively participating in fixpoint calculation.
    #[inline]
    fn entry_is_stale_or_absent<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
        &self,
        cursor: I::Cursor,
        state: &RecursiveState<'src, I::Cursor, E>,
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
    P: Parser<'src, I, A, E> + 'src,
    A: Clone + 'static,
{
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        recursive_growth!(self, input, state, parse, A)
    }
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        recursive_growth!(self, input, state, check, ())
    }
    fn hint(&self) -> ParseHint {
        if let Some(inner) = &self.name() {
            ParseHint::Named {
                name: inner.to_string(),
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
    A: Clone + 'static,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        match self {
            RecursiveInner::Owned(_, owned) => owned.as_ref().parse(input, state),
            RecursiveInner::Unowned(_, weak) => weak.upgrade().map_or_else(
                || {
                    // Parser dropped!
                    let failure = E::Failure::failure_at(
                        input,
                        state.cursor,
                        FailureKind::DroppedParser {
                            name: self.name_cow(),
                        },
                    );
                    (
                        state.clone(),
                        ParseOutput {
                            ast: ParseOutcome::Fatal(failure),
                            cst: AssociatedGreenCst::new(),
                        },
                    )
                },
                |inner| inner.parse(input, state.clone()),
            ),
        }
    }

    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        match self {
            RecursiveInner::Owned(_, owned) => owned.as_ref().check(input, state),
            RecursiveInner::Unowned(_, weak) => weak.upgrade().map_or_else(
                || {
                    // Parser dropped!
                    let failure = E::Failure::failure_at(
                        input,
                        state.cursor,
                        FailureKind::DroppedParser {
                            name: self.name_cow(),
                        },
                    );
                    (
                        state.clone(),
                        ParseOutput {
                            ast: ParseOutcome::Fatal(failure),
                            cst: AssociatedGreenCst::new(),
                        },
                    )
                },
                |inner| inner.check(input, state.clone()),
            ),
        }
    }
}

impl<'p, 'src, I: Input + ?Sized, A, E: ParseEnvironment> Parser<'src, I, A, E>
    for Indirect<'p, 'src, I, A, E>
where
    A: Clone + 'static,
{
    #[inline(always)]
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<A, E>) {
        match self.inner.get() {
            Some(parser) => parser.parse(input, state),
            None => {
                let failure = E::Failure::failure_at(
                    input,
                    state.cursor,
                    FailureKind::UndefinedParser {
                        name: self.name.clone().map(Into::into),
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Fatal(failure),
                        cst: AssociatedGreenCst::new(),
                    },
                )
            }
        }
    }

    #[inline(always)]
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E>,
    ) -> (ParseState<'src, I, E>, ParseOutput<(), E>) {
        match self.inner.get() {
            Some(parser) => parser.check(input, state),
            None => {
                let failure = E::Failure::failure_at(
                    input,
                    state.cursor,
                    FailureKind::UndefinedParser {
                        name: self.name.clone().map(Into::into),
                    },
                );
                (
                    state,
                    ParseOutput {
                        ast: ParseOutcome::Fatal(failure),
                        cst: AssociatedGreenCst::new(),
                    },
                )
            }
        }
    }
}
