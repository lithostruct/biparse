//! Optional implementation of [`Pattern`] for [`regex::Regex`] and [`regex_lite::Regex`].

use crate::input::{Input, Pattern, PatternMatch, str::StrIndex};

impl Pattern<str> for regex::Regex {
    type Payload<'src> = &'src str;

    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        match self.find_at(input, cursor.into_usize()) {
            Some(matched) => if matched.start() == cursor.into_usize() {
                Ok(PatternMatch::new(
                    matched.as_str(),
                    vec![matched.range()],
                    StrIndex(matched.range().end))
                )
            } else {
                // There isn't really a meaningful "error at" cursor we can provide here,
                // so return the current cursor.
                Err(cursor)
            },
            None => Err(cursor),
        }
    }

    fn describe(&self) -> String {
        format!("regex pattern {:?}", self.as_str())
    }
}
impl Pattern<str> for regex_lite::Regex {
    type Payload<'src> = &'src str;

    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        // Literally identical to above.
        match self.find_at(input, cursor.into_usize()) {
            Some(matched) => if matched.start() == cursor.into_usize() {
                Ok(PatternMatch::new(
                    matched.as_str(),
                    vec![matched.range()],
                    StrIndex(matched.range().end))
                )
            } else {
                // There isn't really a meaningful "error at" cursor we can provide here,
                // so return the current cursor.
                Err(cursor)
            },
            None => Err(cursor),
        }
    }

    fn describe(&self) -> String {
        format!("regex pattern {:?}", self.as_str())
    }
}