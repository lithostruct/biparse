// use crate::{
//     green::green_interner::GreenElement,
//     parser::{
//         common::{Parser, ParserOutput, ParserResult, ParserState},
//         cst::Zero,
//     },
// };

// /// Consumes no input and succeeds immediately.
// ///
// /// The zero identity for sequential operations.
// pub struct Empty;
// impl<F, Id> Parser<(), F, Id, Zero<GreenElement>> for Empty {
//     fn parse<I: crate::input::Input>(
//         &self,
//         input: &I,
//         state: ParserState<I>,
//     ) -> (ParserState<I>, ParserOutput<(), F, Id, Zero<GreenElement>>) {
//         (state, ParserOutput {
//             ast: ParserResult::Success(()),
//             cst: Zero::new(),
//             origin: im::HashMap::new(),
//             errors: im::HashMap::new(),
//         })
//     }

//     fn check<I: crate::input::Input>(
//         &self,
//         input: &I,
//         state: ParserState<I>,
//     ) -> (ParserState<I>, ParserOutput<(), F, Id, ()>) {
//         // Identical to parse, other than the CST payload.
//         let (state, output) = self.parse(input, state);
//         (state, ParserOutput {
//             ast: output.ast,
//             cst: (),
//             origin: im::HashMap::new(),
//             errors: output.errors,
//         })
//     }
// }
