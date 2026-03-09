# lexer stub

Purpose: tokenize `.as` / `.asi` source into lexical tokens.

Planned API:
- `tokenize(source: &str) -> Vec<Token>`
- `TokenKind` for keywords, identifiers, literals, operators, delimiters
- span tracking for Mwalimu diagnostics
