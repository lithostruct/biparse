//! Optional implementation of [`Pattern`] for [`regex::Regex`] and [`regex_lite::Regex`].

use rowan::SyntaxKind;

use crate::input::{Input, Pattern, PatternMatch, str::StrIndex};

impl<K> Pattern<str> for (K, regex::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src> = &'src str;

    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
        match self.1.find_at(input, cursor.into_usize()) {
            Some(matched) => {
                if matched.start() == cursor.into_usize() {
                    Ok(PatternMatch::new(
                        matched.as_str(),
                        vec![(self.0.clone().into(), matched.range())],
                        StrIndex(matched.range().end),
                    ))
                } else {
                    // There isn't really a meaningful "error at" cursor we can provide here.
                    Err(None)
                }
            }
            None => Err(None),
        }
    }

    fn describe(&self) -> String {
        format!("regex pattern {:?}", self.1.as_str())
    }
}

impl<K> Pattern<str> for (K, regex_lite::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src> = &'src str;

    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
        match self.1.find_at(input, cursor.into_usize()) {
            Some(matched) => {
                if matched.start() == cursor.into_usize() {
                    Ok(PatternMatch::new(
                        matched.as_str(),
                        vec![(self.0.clone().into(), matched.range())],
                        StrIndex(matched.range().end),
                    ))
                } else {
                    // There isn't really a meaningful "error at" cursor we can provide here.
                    Err(None)
                }
            }
            None => Err(None),
        }
    }

    fn describe(&self) -> String {
        format!("regex pattern {:?}", self.1.as_str())
    }
}
