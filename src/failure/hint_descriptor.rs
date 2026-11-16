use ptree::*;
use std::{borrow::Cow, collections::VecDeque, fmt::Display};

/// The type used by [`Parser`](crate::parse::Parser)'s to "hint" at the input they expect.
///
/// Used for [`ParseFailure`](crate::failure::ParseFailure) error construction and reporting.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum ParseHint {
    /// No information known.
    #[default]
    Opaque,

    /// Succeeds unconditionally, consuming nothing.
    Empty,
    /// Fails unconditionally.
    Fails,
    /// Expects something, consuming up to that point.
    Expectation(Descriptor),

    /// Anything but this; consumes no input.
    Not(Box<Self>),
    /// This, but consume no input.
    Peek(Box<Self>),

    /// All of the following in sequential order.
    Sequence(Vec<Self>),
    /// Any of the following as ordered choice.
    Choice(Vec<Self>),
    /// This, for some number of times.
    Repetition {
        /// The minimum number of repetitions, inclusive.
        minimum: usize,
        /// The maximum number of repetitions, inclusive.
        maximum: Option<usize>,
        /// The inner hint.
        inner: Box<Self>,
    },

    /// A named representation of this.
    Named {
        /// The name of this parser.
        name: String,
        /// The inner hint.
        inner: Box<Self>,
    },
}
impl ParseHint {
    /// Constructs [`ParseHint::Sequence`] while eagerly flattening one-layer deep.
    pub fn sequence(children: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Vec::new();
        for child in children {
            if let ParseHint::Sequence(mut nested) = child {
                result.append(&mut nested);
            } else {
                result.push(child);
            }
        }
        ParseHint::Sequence(result)
    }
    /// Constructs [`ParseHint::Choice`] while eagerly flattening one-layer deep.
    pub fn choice(children: impl IntoIterator<Item = Self>) -> Self {
        let mut result = Vec::new();
        for child in children {
            if let ParseHint::Choice(mut nested) = child {
                result.append(&mut nested);
            } else {
                result.push(child);
            }
        }
        ParseHint::Choice(result)
    }

    /// Postorder traversal to canonize a [`ParseHint`], to be used by text renderers.
    pub fn canonize(self) -> Self {
        match self {
            // Leaves.
            ParseHint::Opaque => ParseHint::Opaque,
            ParseHint::Empty => ParseHint::Empty,
            ParseHint::Fails => ParseHint::Fails,

            ParseHint::Expectation(expectation) => ParseHint::Expectation(expectation),
            ParseHint::Not(hint) => {
                match hint.canonize() {
                    // Leaf propagation.
                    ParseHint::Empty => ParseHint::Fails,
                    ParseHint::Fails => ParseHint::Empty,
                    // Inner inversion.
                    ParseHint::Not(inner) => ParseHint::Peek(inner),
                    ParseHint::Peek(inner) => ParseHint::Not(inner),
                    hint => ParseHint::Not(Box::new(hint)),
                }
            }
            ParseHint::Peek(hint) => {
                match hint.canonize() {
                    // Leaf propagation.
                    ParseHint::Empty => ParseHint::Empty,
                    ParseHint::Fails => ParseHint::Fails,
                    // Inner de-nesting.
                    ParseHint::Not(inner) => ParseHint::Not(inner),
                    ParseHint::Peek(inner) => ParseHint::Peek(inner),
                    hint => ParseHint::Peek(Box::new(hint)),
                }
            }
            ParseHint::Repetition { minimum, maximum, inner } => {
                match inner.canonize() {
                    // Leaf propagation.
                    ParseHint::Empty => ParseHint::Empty,
                    ParseHint::Fails => ParseHint::Empty,
                    // Inner de-nesting.
                    ParseHint::Repetition { minimum: min2, maximum: max2, inner } => {
                        let total_minimum =  minimum * min2;
                        let total_maximum = match (maximum, max2) {
                            (Some(inner), Some(inner2)) => Some(inner * inner2),
                            _ => None,
                        };
                        ParseHint::Repetition {
                            minimum: total_minimum,
                            maximum: total_maximum,
                            inner 
                        }
                    },
                    hint => ParseHint::Repetition {
                        minimum,
                        maximum,
                        inner: Box::new(hint),
                    },
                }
            }
            ParseHint::Named { name, inner } => ParseHint::Named {
                name,
                inner: Box::new(inner.canonize()),
            },
            ParseHint::Sequence(hints) => {
                let mut queue: VecDeque<ParseHint> = hints.into();
                let mut result = Vec::new();
                while let Some(front) = queue.pop_front() {
                    match front.canonize() {
                        // Remove empty elements.
                        ParseHint::Empty => (),
                        // Any sequence containing an unconditional failure itself unconditionally fails.
                        ParseHint::Fails => {
                            return ParseHint::Fails;
                        }
                        // De-nesting.
                        ParseHint::Sequence(subarray) => {
                            // Preserving order, push subarray elements into the queue.
                            for item in subarray.into_iter().rev() {
                                queue.push_front(item);
                            }
                        }
                        element => result.push(element),
                    }
                }
                if result.is_empty() {
                    ParseHint::Empty
                } else {
                    ParseHint::Sequence(result)
                }
            }
            ParseHint::Choice(hints) => {
                let mut queue: VecDeque<ParseHint> = hints.into();
                let mut result = Vec::new();
                while let Some(front) = queue.pop_front() {
                    match front.canonize() {
                        // Eliminate failing branches.
                        ParseHint::Fails => (),
                        // De-nesting.
                        ParseHint::Choice(subarray) => {
                            // Preserving order, push subarray elements into the queue.
                            for item in subarray.into_iter().rev() {
                                queue.push_front(item);
                            }
                        }
                        element => result.push(element),
                    }
                }
                if result.is_empty() {
                    // This indicates that there exists no valid alternatives to take,
                    // usually because all alternatives canonized to `Fails`.
                    ParseHint::Fails
                } else {
                    ParseHint::Choice(result)
                }
            }
        }
    }

    /// Returns `self` as a string containing a structured tree representation.
    pub fn to_tree(&self) -> String {
        self.to_tree_with(&ptree::PrintConfig::default(), false)
    }
    /// Returns `self` as a string containing a structured tree representation,
    /// applying the provided [`ptree::PrintConfig`].
    ///
    /// The `verbose` flag specifies whether named hints should be
    /// treated as terminals ([`false`]) or non-terminals ([`true`]).
    pub fn to_tree_with(&self, config: &ptree::PrintConfig, verbose: bool) -> String {
        let mut store = Vec::<u8>::new();
        if verbose {
            ptree::write_tree_with(ExpandedHint::expand(self), &mut store, config).unwrap();
        } else {
            ptree::write_tree_with(self, &mut store, config).unwrap();
        }
        String::from_utf8(store).unwrap()
    }
    /// Returns `self` as an structured expectation tree.
    ///
    /// Differs from [`Self::to_tree`] by attaching an `"expected:"` ancestor node.
    pub fn to_expectation(&self) -> String {
        self.to_expectation_with(&ptree::PrintConfig::default(), false)
    }
    /// Returns `self` as an structured expectation tree,
    /// applying the provided [`ptree::PrintConfig`].
    ///
    /// Differs from [`Self::to_tree_with`] by attaching an `"expected:"` ancestor node.
    ///
    /// The `verbose` flag specifies whether named hints should be
    /// treated as terminals ([`false`]) or non-terminals ([`true`]).
    pub fn to_expectation_with(&self, config: &PrintConfig, verbose: bool) -> String {
        let mut store = Vec::<u8>::new();
        if verbose {
            ptree::write_tree_with(&Expected(ExpandedHint::expand(self)), &mut store, config)
                .unwrap();
        } else {
            ptree::write_tree_with(&Expected(self), &mut store, config).unwrap();
        }
        String::from_utf8(store).unwrap()
    }
}

impl TreeItem for ParseHint {
    type Child = Self;

    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &Style) -> std::io::Result<()> {
        let text = match self {
            ParseHint::Opaque => "something",
            ParseHint::Empty => "nothing",
            ParseHint::Fails => "fails",
            ParseHint::Expectation(descriptor) => &descriptor.to_string(),
            ParseHint::Not(_) => "not:",
            ParseHint::Peek(_) => "peek:",
            ParseHint::Sequence(_) => "all of:",
            ParseHint::Choice(_) => "any of:",
            ParseHint::Repetition { minimum, maximum, inner: _ } => {
                &format!("repeat ({minimum}..{}):", maximum.map_or_default(|max| format!("={max}")))
            },
            ParseHint::Named { name, inner: _ } => name,
        };
        write!(f, "{}", style.paint(text))
    }
    fn children(&self) -> Cow<'_, [Self::Child]> {
        match self {
            // Terminals
            ParseHint::Opaque
            | ParseHint::Empty
            | ParseHint::Fails
            | ParseHint::Expectation(_)
            // Named is treated as a terminal.
            | ParseHint::Named { .. } => {
                Cow::Owned(Vec::new())
            },

            // Single child.
            ParseHint::Not(hint)
            | ParseHint::Peek(hint)
            | ParseHint::Repetition{inner: hint, ..} => {
                Cow::Borrowed(std::slice::from_ref(hint.as_ref()))
            },

            // Multiple children.
            ParseHint::Sequence(hints)
            | ParseHint::Choice(hints) => {
                Cow::Borrowed(hints.as_slice())
            },
        }
    }
}

/// A transparent newtype for prepending `"expected:"` as an ancestor to the enclosed `ParseHint` or `ExpandedHint`.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct Expected<'a, T>(&'a T);
impl<'a, T> TreeItem for Expected<'a, T>
where
    T: TreeItem,
{
    type Child = T;
    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &Style) -> std::io::Result<()> {
        write!(f, "{}", style.paint("expected:"))
    }
    fn children(&self) -> Cow<'_, [Self::Child]> {
        Cow::Borrowed(std::slice::from_ref(self.0))
    }
}

/// A transparent newtype for expanding the `ptree::TreeItem` implementation to look into named hints.
#[derive(Debug, Clone)]
#[repr(transparent)]
struct ExpandedHint(ParseHint);
impl ExpandedHint {
    /// Converts a single reference into expanded form.
    fn expand(hint: &ParseHint) -> &Self {
        // SAFETY:
        // - Pointer is non-null and points to `ParseHint` data.
        // - Lifetime of the borrow is unchanged, and the underlying data may not be mutated.
        // - Transparent wrapper results in identical layout and alignment.
        unsafe { &*(hint as *const ParseHint as *const ExpandedHint) }
    }
    /// Converts a slice of references into expanded form.
    fn expand_slice(children: &[ParseHint]) -> &[Self] {
        // SAFETY:
        // - Pointer is non-null and points to `ParseHint` slice data.
        // - Lifetime of the borrow is unchanged, and the underlying data may not be mutated.
        // - Transparent wrapper results in identical layout and alignment for each element.
        // - The length of the slice is unchanged, and is no larger than `isize::MAX`.
        unsafe {
            std::slice::from_raw_parts(children.as_ptr() as *const ExpandedHint, children.len())
        }
    }
}
impl TreeItem for ExpandedHint {
    type Child = Self;
    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &Style) -> std::io::Result<()> {
        // None of the lines changed, so we delegate to [`ParseHint`].
        self.0.write_self(f, style)
    }
    fn children(&self) -> Cow<'_, [Self::Child]> {
        match &self.0 {
            // Terminals
            ParseHint::Opaque
            | ParseHint::Empty
            | ParseHint::Fails
            | ParseHint::Expectation(_) => {
                Cow::Owned(Vec::new())
            },

            // Single child.
            ParseHint::Not(hint)
            | ParseHint::Peek(hint)
            | ParseHint::Repetition{inner: hint, ..}
            // Named is treated as a non-terminal.
            | ParseHint::Named { name: _, inner: hint } => {
                Cow::Borrowed(ExpandedHint::expand_slice(
                    std::slice::from_ref(hint.as_ref())
                ))
            },

            // Multiple children.
            ParseHint::Sequence(hints)
            | ParseHint::Choice(hints) => {
                Cow::Borrowed(ExpandedHint::expand_slice(
                    hints.as_slice()
                ))
            },
        }
    }
}

/// A textual representation of an expected item or token.
///
/// Used for [`ParseFailure`](crate::failure::ParseFailure) error construction and reporting.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Descriptor {
    /// A primitive pattern.
    Pattern {
        /// The kind of this pattern. Must be singular, all-lowercase.
        kind: String,
        /// The kind of this pattern. Must be plural, all-lowercase.
        kind_plural: String,
        /// The data of this pattern.
        data: String,
    },
    /// End-of-input.
    EndOfInput,
    /// Any pattern.
    Any,
}
impl Display for Descriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Descriptor::Pattern {
                kind,
                kind_plural: _,
                data,
            } => write!(f, "{kind} {data}"),
            Descriptor::EndOfInput => write!(f, "end of input"),
            Descriptor::Any => write!(f, "any input"),
        }
    }
}
