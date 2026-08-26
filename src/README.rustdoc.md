Biparse is a parser-combinator framework designed around dual AST + CST construction and correspondence.  
Write an AST parser, get the CST for free! No more tree-walking over a `rowan` red tree!

# Quick Start

# Motivation

Writing a source to CST to AST parser by hand is fine only if you do it once only.

Tools like `rowan` are as useful as they are primitive, and one typically needs to write *two* parsing passes: once to convert a lexer stream to a CST, then another to turn a CST to an AST. The latter pass introduces a whole host of logical bugs and headaches, and the two passes tend to be closely interconnected, making iteration time-consuming and bug-prone. Most "move fast, break things" language exploration tends to ditch the CST altogether.

Make no mistake, `rowan` is good! In fact, this crate uses it for its CST representation!  
But surely we can do better than hand-written parsers, no?

With minimal bookkeeping, an otherwise source-to-AST parser can get a concrete syntax tree, and the correspondence mechanism allows for AST nodes to embed an identity value that can later be used to track down its CST equivalent.