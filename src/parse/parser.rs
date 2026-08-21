use crate::{
    cst::{CstOutput, IdSupply},
    input::Input,
    parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState},
};

pub trait Parser<'src, I: Input, A, E: ParseEnvironment = DefaultParseEnvironment> {
    /// Parse the input.
    fn parse<S: IdSupply<Id = E::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (ParseState<I, S>, ParseOutput<A, E::Failure, E::Id>);

    /// Checks the input for any errors.
    ///
    /// AST, CST and origin information is discarded. CST information *must* be empty (equivalent to [`CstOutput::new`]).
    fn check<S: IdSupply<Id = E::Id>>(
        &self,
        input: &'src I,
        state: ParseState<I, S>,
    ) -> (ParseState<I, S>, ParseOutput<(), E::Failure, E::Id>) {
        let (new_state, output) = self.parse(input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: CstOutput::new(),
        };
        (new_state, erased_output)
    }
}
