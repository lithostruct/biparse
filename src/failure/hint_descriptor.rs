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
    /// This, zero or more times.
    Many(Box<Self>),

    /// A named representation of this.
    Named { name: String, inner: Box<Self> },
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
            ParseHint::Many(hint) => {
                match hint.canonize() {
                    // Leaf propagation.
                    ParseHint::Empty => ParseHint::Empty,
                    ParseHint::Fails => ParseHint::Empty,
                    // Inner de-nesting.
                    ParseHint::Many(inner) => ParseHint::Many(inner),
                    hint => ParseHint::Many(Box::new(hint)),
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
        self.to_tree_with(&ptree::PrintConfig::default())
    }
    /// Returns `self` as a string containing a structured tree representation, applying the provided [`ptree::PrintConfig`].
    pub fn to_tree_with(&self, config: &ptree::PrintConfig) -> String {
        let mut store = Vec::<u8>::new();
        ptree::write_tree_with(self, &mut store, config).unwrap();
        String::from_utf8(store).unwrap()
    }
    /// Returns `self` as an structured expectation tree.
    /// 
    /// Differs from [`Self::to_tree`] by attaching an `"expected:"` ancestor node.
    pub fn to_expectation_tree(&self) -> String {
        self.to_expectation_tree_with(&ptree::PrintConfig::default())
    }
    /// Returns `self` as an structured expectation tree.
    /// 
    /// Differs from [`Self::to_tree_with`] by attaching an `"expected:"` ancestor node.
    pub fn to_expectation_tree_with(&self, config: &ptree::PrintConfig) -> String {
        let mut store = Vec::<u8>::new();
        ptree::write_tree_with(&Expected(self), &mut store, config).unwrap();
        String::from_utf8(store).unwrap()
    }
}

impl ptree::TreeItem for ParseHint {
    type Child = Self;

    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &ptree::Style) -> std::io::Result<()> {
        let text = match self {
            ParseHint::Opaque => "something",
            ParseHint::Empty => "nothing",
            ParseHint::Fails => "fails",
            ParseHint::Expectation(descriptor) => &descriptor.to_string(),
            ParseHint::Not(_) => "not:",
            ParseHint::Peek(_) => "peek:",
            ParseHint::Sequence(_) => "all of:",
            ParseHint::Choice(_) => "any of:",
            ParseHint::Many(_) => "zero or more:",
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
            | ParseHint::Expectation(_) => {
                Cow::Owned(Vec::new())
            },
            // Named is treated as a terminal.
            ParseHint::Named { .. } => {
                Cow::Owned(Vec::new())
            },

            // Single child.
            ParseHint::Not(hint)
            | ParseHint::Peek(hint)
            | ParseHint::Many(hint) => {
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

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
struct Expected<'a>(&'a ParseHint);
impl<'a> ptree::TreeItem for Expected<'a> {
    type Child = ParseHint;
    fn write_self<W: std::io::Write>(&self, f: &mut W, style: &ptree::Style) -> std::io::Result<()> {
        write!(f, "{}", style.paint("expected:"))
    }
    fn children(&self) -> Cow<'_, [Self::Child]> {
        Cow::Borrowed(std::slice::from_ref(self.0))
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
    EndOfInput,
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
