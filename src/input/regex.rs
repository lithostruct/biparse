//! Optional implementation of [`Pattern`] for [`regex::Regex`] and [`regex_lite::Regex`].

#![cfg(feature = "regex")]

use rowan::SyntaxKind;

use crate::{
    input::{Input, Pattern, PatternMatch, PatternMatchResult, str::StrIndex},
    parse::{Expectation, ParseHint},
};

/// `regex` support.
impl<'src, K> Pattern<'src, str> for (K, regex::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload = &'src str;

    fn matches_at(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, str> {
        match self.1.find_at(input, cursor.into_usize()) {
            Some(matched) => {
                if matched.start() == cursor.into_usize() {
                    Ok(PatternMatch {
                        payload: matched.as_str(),
                        cst_spans: vec![(self.0.clone().into(), matched.range())],
                        next_cursor: StrIndex(matched.range().end),
                    })
                } else {
                    // There isn't really a meaningful "error at" cursor we can provide here.
                    Err(None)
                }
            }
            None => Err(None),
        }
    }

    fn describe(&self) -> String {
        format!("regex {:?}", self.1.as_str())
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Expectation::Pattern {
            kind: "regex".to_string(),
            kind_plural: "regexes".to_string(),
            data: format!("{:?}", self.1.as_str()),
        })
    }
}

/// `regex_lite` support.
impl<'src, K> Pattern<'src, str> for (K, regex_lite::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload = &'src str;

    fn matches_at(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, str> {
        match self.1.find_at(input, cursor.into_usize()) {
            Some(matched) => {
                if matched.start() == cursor.into_usize() {
                    Ok(PatternMatch {
                        payload: matched.as_str(),
                        cst_spans: vec![(self.0.clone().into(), matched.range())],
                        next_cursor: StrIndex(matched.range().end),
                    })
                } else {
                    // There isn't really a meaningful "error at" cursor we can provide here.
                    Err(None)
                }
            }
            None => Err(None),
        }
    }

    fn describe(&self) -> String {
        format!("regex {:?}", self.1.as_str())
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Expectation::Pattern {
            kind: "regex".to_string(),
            kind_plural: "regexes".to_string(),
            data: format!("{:?}", self.1.as_str()),
        })
    }
}
