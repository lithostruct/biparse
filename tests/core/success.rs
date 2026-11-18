//! Tests for the [`Success`](biparse::combinator::Success) primitive combinator.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};
use rowan::SyntaxKind;

proptest! {
    #[test]
    fn single_empty(s in any::<String>()) {
        let (state, output): (_, ParseOutput<(), environment::Rich>) = success()
            .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        assert_eq!(state.cursor.into_usize(), 0);
        assert_eq!(output.ast, ParseOutcome::Success(()));
    }

    #[test]
    fn empty_then_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<&str, environment::Rich>)
            = just((SyntaxKind(kind), pattern.clone()))
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<&str, environment::Rich>)
            = success()
                .then(just((SyntaxKind(kind), pattern)))
                .map(|(_, s)| s)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn just_then_empty(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<&str, environment::Rich>)
            = just((SyntaxKind(kind), pattern.clone()))
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<&str, environment::Rich>)
            = just((SyntaxKind(kind), pattern))
                .then(success()).map(|(s, _)| s)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn empty_or_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<Either<(), &str>, environment::Rich>)
            = success().map(Either::Left)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<Either<(), &str>, environment::Rich>)
            = success()
                .either_or(just((SyntaxKind(kind), pattern)))
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn just_or_empty(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        // Currently no way of recovering from a failed `just` to get an `Either::Right(())`,
        // so this is more of a stress test than anything.
        let (state, output): (_, ParseOutput<Either<&str, ()>, environment::Rich>)
            = just((SyntaxKind(kind), pattern.clone()))
                .either_or(success())
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        let ParseOutcome::Success(inner) = output.ast else { unreachable!() };
        match inner {
            Either::Left(atom) => assert_eq!(state.cursor.into_usize(), atom.len()),
            Either::Right(()) => assert_eq!(state.cursor.into_usize(), 0),
        }
    }
}
