use std::{collections::VecDeque, fmt::Display};

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
