// //! Test relating to `CachedStr` input type.

// use biparse::{input::CachedStr, prelude::*};
// use proptest::{arbitrary::any, proptest};
// use rowan::SyntaxKind;

// // Identicality assertions.
// proptest! {
//     #[test]
//     fn arbitrary_char_pattern(pat in any::<char>(), s in any::<String>()) {
//         let str_parser =
//             just::<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), pat))
//                 .to_owned();
//         let cachedstr_parser =
//             just::<'_, CachedStr<'_>, DefaultParseEnvironment, _>((SyntaxKind(0), pat))
//                 .to_owned();

//         let str_output = str_parser.parse_from_start(&s);
//         let cachedstr = CachedStr::new(&s);
//         let cachedstr_output = cachedstr_parser.parse_from_start(&cachedstr);

//         assert_eq!(str_output.1, cachedstr_output.1);
//         assert_eq!(str_output.0.cursor, cachedstr_output.0.cursor);
//     }
//     #[test]
//     fn arbitrary_string_pattern(pat in any::<String>(), s in any::<String>()) {
//         let str_parser =
//             just::<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), pat.clone()))
//                 .to_owned();
//         let cachedstr_parser =
//             just::<'_, CachedStr<'_>, DefaultParseEnvironment, _>((SyntaxKind(0), pat))
//                 .to_owned();

//         let str_output = str_parser.parse_from_start(&s);
//         let cachedstr = CachedStr::new(&s);
//         let cachedstr_output = cachedstr_parser.parse_from_start(&cachedstr);

//         assert_eq!(str_output.1, cachedstr_output.1);
//         assert_eq!(str_output.0.cursor, cachedstr_output.0.cursor);
//     }
// }
