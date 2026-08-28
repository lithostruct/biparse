use std::fmt::Display;

/// The diagnostics information for
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum ParseHint {
    /// No information known.
    #[default]
    Opaque,

    /// Succeeds unconditionally, consuming nothing.
    Empty,
    /// Fails unconditionally.
    Fails,
    /// Expects something.
    Expectation(Expectation),

    /// Anything but this.
    Not(Box<Self>),
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expectation {
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
impl Display for Expectation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expectation::Pattern {
                kind,
                kind_plural: _,
                data,
            } => write!(f, "{kind} {data}"),
            Expectation::EndOfInput => write!(f, "end of input"),
            Expectation::Any => write!(f, "any input"),
        }
    }
}
