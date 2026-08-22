use crate::{
    cst::CstOutput,
    input::Input,
    parse::{DefaultParseEnvironment, ParseEnvironment, ParseOutput, ParseState},
};

/// The central parsing trait, implemented by all combinators.
pub trait Parser<'src, I: Input, A, E: ParseEnvironment = DefaultParseEnvironment> {
    /// Parse the input.
    fn parse(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<A, E>);

    /// Checks the input for any errors.
    ///
    /// AST, CST and origin information is discarded. CST information *must* be empty (equivalent to [`CstOutput::new`]).
    fn check(
        &self,
        input: &'src I,
        state: ParseState<'src, I, E::Supply>,
    ) -> (ParseState<'src, I, E::Supply>, ParseOutput<(), E>) {
        let (new_state, output) = self.parse(input, state);
        let erased_output = ParseOutput {
            ast: output.ast.map(|_| ()),
            cst: CstOutput::new(),
        };
        (new_state, erased_output)
    }
}
