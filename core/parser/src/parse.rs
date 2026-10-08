//! Parsing: module items and simple statements here; blocks, expressions and patterns in the
//! flat machines of [`block`], [`expr`] and [`pattern`]. Nothing in the parser recurses: nesting
//! lives on explicit heap stacks, so source nesting never costs machine stack.

mod block;
mod expr;
mod pattern;

use asili_diagnostics::Diagnostic;
use std::mem;

use crate::cursor::Parser;
use crate::{
    AssignOp, Attribute, BinaryOp, Constant, EnumDecl, EnumVariant, Expr, ExprId, Exprs, Function,
    ImplDecl, Import, ImportPath, Module, Param, Stmt, StructDecl, TraitDecl, TraitMethodSig,
    TypeExpr,
};
use asili_lexer::{tk, TokenKind};

/// Deepest nesting of blocks and brackets a program may use (`PAR073`). The parser itself has no
/// limit — its stacks live on the heap — and expressions are stored flat (`Exprs`), but the
/// passes after it (the semantic analyzer, the bytecode compiler, the LSP) still walk the tree
/// recursively, and nested blocks are serialized and dropped recursively.
const MAX_NESTING: usize = 1000;

/// Strip surrounding double quotes from string literal lexeme so the AST holds content only.
fn strip_string_lexeme_quotes(lexeme: &str) -> String {
    if lexeme.len() >= 2 && lexeme.starts_with('"') && lexeme.ends_with('"') {
        lexeme[1..lexeme.len() - 1].to_string()
    } else {
        lexeme.to_string()
    }
}

impl<'a> Parser<'a> {
    pub(crate) fn parse_module(&mut self) -> Module {
        let mut imports = Vec::new();
        let mut constants = Vec::new();
        let mut enums = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut traits = Vec::new();
        let mut impls = Vec::new();
        let mut pending_test = false;
        let mut pending_attrs: Vec<Attribute> = Vec::new();

        while !self.is_eof() {
            if self.check(tk!("#")) && self.check_n(1, tk!("[")) {
                if let Some(attr) = self.parse_attribute() {
                    pending_test = attr.name == "jaribio";
                    pending_attrs.push(attr);
                } else {
                    self.pos += 1;
                }
                continue;
            }

            if self.match_tok(tk!("leta")) {
                let line = self.prev().line;
                if let Some(path) = self.parse_import_path() {
                    imports.push(Import { path, line });
                }
                continue;
            }

            if self.match_tok(tk!("thabiti")) {
                let line = self.prev().line;
                if let Some(const_decl) = self.parse_module_constant(line) {
                    constants.push(const_decl);
                }
                continue;
            }

            let is_public = self.match_tok(tk!("umma"));
            if self.match_tok(tk!("kazi")) {
                if let Some(func) =
                    self.parse_function(is_public, pending_test, mem::take(&mut pending_attrs))
                {
                    functions.push(func);
                }
                pending_test = false;
                continue;
            }

            if self.match_tok(tk!("jenum")) {
                if let Some(decl) = self.parse_enum_decl(is_public, mem::take(&mut pending_attrs)) {
                    enums.push(decl);
                }
                continue;
            }

            if self.match_tok(tk!("umbo")) {
                if let Some(decl) = self.parse_struct_decl(is_public, mem::take(&mut pending_attrs))
                {
                    structs.push(decl);
                }
                continue;
            }

            if self.match_tok(tk!("sifa")) {
                if let Some(decl) = self.parse_trait_decl(is_public, mem::take(&mut pending_attrs))
                {
                    traits.push(decl);
                }
                continue;
            }

            if self.match_tok(tk!("shughuli")) {
                if self.match_tok(tk!("ya")) {
                    if let Some(decl) = self.parse_impl_decl(mem::take(&mut pending_attrs)) {
                        impls.push(decl);
                    }
                } else {
                    self.err_here("PAR901", "shughuli inahitaji 'ya'");
                    self.skip_top_level();
                }
                continue;
            }

            if self.match_tok(tk!(";")) {
                continue;
            }
            // Right after an item that failed to parse, its own error already points here.
            let here = (self.peek().line, self.peek().column);
            let reported = self
                .errors
                .last()
                .and_then(|d| d.span.as_ref())
                .is_some_and(|sp| (sp.line, sp.column) == here);
            if !reported {
                self.err_here("PAR000", "token isiyotegemewa kwenye kiwango cha juu");
            }
            self.pos += 1;
            self.skip_top_level();
        }

        enums.extend(self.standard_enums());
        traits.extend(self.standard_traits());

        Module {
            exprs: std::mem::take(&mut self.exprs),
            imports,
            constants,
            enums,
            functions,
            structs,
            traits,
            impls,
        }
    }

    fn parse_module_constant(&mut self, line: usize) -> Option<Constant> {
        let name_tok = self.consume_ident("PAR040", "thabiti inahitaji jina")?;
        let name = name_tok.lexeme.clone();
        let column = name_tok.column;
        // Explicit type annotation is optional (`thabiti X = 1` vs `thabiti X: Namba = 1`), but
        // the leading ':' must actually be consumed when present — parse_type() has no special
        // handling for a leading ':' itself (":" isn't one of its stop-tokens), so without this
        // `match_tok`, a typed constant like `thabiti PI: Namba = 3.14` silently parsed its type
        // as the bogus string ": Namba" instead of "Namba", which no ValueType recognizes, so it
        // always resolved to ValueType::Unknown regardless of what was actually written.
        let ty = if self.match_tok(tk!(":")) {
            self.parse_type()
        } else {
            TypeExpr {
                name: String::new(),
            }
        };
        self.consume(tk!("="), "PAR041", "thabiti inahitaji '='")?;
        let value = self.parse_expression()?;
        Some(Constant {
            name,
            ty,
            value,
            line,
            column,
        })
    }

    fn parse_import_path(&mut self) -> Option<ImportPath> {
        let module = self
            .consume_ident("PAR059", "leta inahitaji jina la moduli")?
            .lexeme
            .clone();
        if self.match_tok(tk!("::")) && self.match_tok(tk!("{")) {
            let mut names = Vec::new();
            loop {
                let t = self.consume_ident("PAR061", "leta ya kuchagua inahitaji jina")?;
                names.push(t.lexeme.clone());
                if self.match_tok(tk!("}")) {
                    break;
                }
                self.consume(
                    tk!(","),
                    "PAR062",
                    "leta ya kuchagua inahitaji ',' kati ya majina",
                )?;
            }
            return Some(ImportPath::Selective { module, names });
        }
        Some(ImportPath::Full(module))
    }

    fn parse_attribute(&mut self) -> Option<Attribute> {
        self.consume(tk!("#"), "PAR080", "kiambatanisho inahitaji '#'")?;
        self.consume(tk!("["), "PAR081", "kiambatanisho inahitaji '['")?;
        let name = self.consume_ident("PAR082", "kiambatanisho inahitaji jina")?;
        let mut args = None;
        if self.match_tok(tk!("(")) {
            let mut raw = String::new();
            while !self.is_eof() && !self.check(tk!(")")) {
                if !raw.is_empty() {
                    raw.push(' ');
                }
                raw.push_str(&self.advance().lexeme);
            }
            self.consume(tk!(")"), "PAR083", "kiambatanisho hoja inahitaji ')'")?;
            args = Some(raw);
        }
        self.consume(tk!("]"), "PAR084", "kiambatanisho inahitaji ']'")?;
        Some(Attribute {
            name: name.lexeme.clone(),
            args,
            line: name.line,
        })
    }

    fn parse_generic_names(&mut self) -> Vec<String> {
        let mut gens = Vec::new();
        if !self.match_tok(tk!("<")) {
            return gens;
        }
        loop {
            if let Some(id) = self.consume_ident("PAR085", "jumla inahitaji jina") {
                gens.push(id.lexeme.clone());
            }
            if self.match_tok(tk!(",")) {
                continue;
            }
            break;
        }
        let _ = self.consume(tk!(">"), "PAR086", "orodha ya jumla inahitaji '>'");
        gens
    }

    fn parse_struct_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<StructDecl> {
        let name = self.consume_ident("PAR902", "umbo inahitaji jina")?;
        let line = name.line;
        let column = name.column;
        let generics = self.parse_generic_names();
        let fields = if self.match_tok(tk!("{")) {
            let mut flds = Vec::new();
            loop {
                if self.match_tok(tk!("}")) {
                    break;
                }
                let field_name = self.consume_ident("PAR902", "umbo inahitaji jina la uga")?;
                let ty = if self.match_tok(tk!(":")) {
                    Some(self.parse_type())
                } else {
                    None
                };
                flds.push((field_name.lexeme.clone(), ty));
                if !self.match_tok(tk!(",")) {
                    // Fields may be newline-separated instead of comma-separated (e.g.
                    // `umbo P { x: Namba\n y: Namba }`) — only treat it as the end of the
                    // field list if '}' actually follows; otherwise loop for the next field.
                    if self.match_tok(tk!("}")) {
                        break;
                    }
                    continue;
                }
            }
            flds
        } else {
            Vec::new()
        };
        Some(StructDecl {
            name: name.lexeme.clone(),
            generics,
            fields,
            line,
            column,
            attrs,
            is_public,
        })
    }

    fn parse_enum_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<EnumDecl> {
        let name = self.consume_ident("PAR905", "jenum inahitaji jina")?;
        let line = name.line;
        let column = name.column;
        let generics = self.parse_generic_names();
        let variants = if self.match_tok(tk!("{")) {
            let mut vars = Vec::new();
            loop {
                if self.match_tok(tk!("}")) {
                    break;
                }
                let var_name = self.consume_ident("PAR905", "jenum inahitaji jina la kigezo")?;
                let var_line = var_name.line;
                let var_column = var_name.column;
                let data = if self.match_tok(tk!("(")) {
                    let ty = self.parse_type();
                    self.consume(tk!(")"), "PAR905", "kigezo inahitaji ')'")?;
                    Some(ty)
                } else {
                    None
                };
                vars.push(EnumVariant {
                    name: var_name.lexeme.clone(),
                    data,
                    line: var_line,
                    column: var_column,
                });
                if !self.match_tok(tk!(",")) {
                    let _ = self.consume(tk!("}"), "PAR905", "jenum inahitaji '}'");
                    break;
                }
            }
            vars
        } else {
            Vec::new()
        };
        Some(EnumDecl {
            name: name.lexeme.clone(),
            generics,
            variants,
            line,
            column,
            is_public,
            attrs,
        })
    }

    fn parse_trait_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<TraitDecl> {
        let name = self.consume_ident("PAR903", "sifa inahitaji jina")?;
        let line = name.line;
        let column = name.column;
        let methods = self.parse_trait_method_sigs();
        Some(TraitDecl {
            name: name.lexeme.clone(),
            methods,
            line,
            column,
            attrs,
            is_public,
        })
    }

    /// Parse a trait body: zero or more bodiless `kazi name(params) -> ReturnType` signatures
    /// inside `{ }`. No `{ }` at all (e.g. a forward-declared/empty trait) yields no methods.
    fn parse_trait_method_sigs(&mut self) -> Vec<TraitMethodSig> {
        let mut methods = Vec::new();
        if !self.match_tok(tk!("{")) {
            return methods;
        }
        while self.match_tok(tk!("kazi")) {
            let Some(name_tok) = self.consume_ident("PAR906", "njia ya sifa inahitaji jina") else {
                break;
            };
            let line = name_tok.line;
            if self
                .consume(tk!("("), "PAR907", "njia ya sifa inahitaji '('")
                .is_none()
            {
                break;
            }
            let params = self.parse_params();
            if self
                .consume(
                    tk!(")"),
                    "PAR908",
                    "njia ya sifa inahitaji ')' baada ya hoja",
                )
                .is_none()
            {
                break;
            }
            let return_type = if self.match_tok(tk!("->")) {
                self.parse_type()
            } else {
                TypeExpr {
                    name: "Tupu".to_string(),
                }
            };
            methods.push(TraitMethodSig {
                name: name_tok.lexeme.clone(),
                params,
                return_type,
                line,
            });
            if self.check(tk!("}")) {
                break;
            }
        }
        let _ = self.match_tok(tk!("}"));
        methods
    }

    fn parse_impl_decl(&mut self, attrs: Vec<Attribute>) -> Option<ImplDecl> {
        let first = self.consume_ident("PAR904", "shughuli ya inahitaji jina la aina")?;
        let line = first.line;
        let mut trait_name = None;
        let target = first.lexeme.clone();
        if self.match_tok(tk!("kwa")) {
            // "shughuli ya Target kwa Trait { }" — kwa keyword syntax. `first` (before `kwa`)
            // is the type being implemented on (already defaulted into `target` above); the
            // identifier after `kwa` is the trait name. Was previously swapped (trait_name set
            // to `first`, target overwritten with the post-`kwa` identifier), which meant a
            // trait-impl's `target` never actually matched its struct's name anywhere method
            // dispatch looks it up — see docs/language/07-mfumo-wa-aina.md's now-resolved
            // "known bug" note.
            if let Some(t) = self.consume_ident("PAR905", "shughuli ya kwa inahitaji jina la sifa")
            {
                trait_name = Some(t.lexeme.clone());
            }
        } else if self.match_tok(tk!(":")) {
            // "shughuli ya Target: Trait { }" — colon syntax
            if let Some(t) = self.consume_ident("PAR905", "shughuli ya : inahitaji jina la sifa") {
                trait_name = Some(t.lexeme.clone());
            }
        }
        let mut body = Vec::new();
        if self.match_tok(tk!("{")) {
            while self.match_tok(tk!("kazi")) {
                if let Some(f) = self.parse_function(false, false, vec![]) {
                    if f.name == "kuu" {
                        self.errors.push(
                            Diagnostic::new("PAR077", "kazi kuu haiwezi kuwa ndani ya shughuli")
                                .with_stage("uchanganuzi")
                                .with_span(f.line, 1),
                        );
                    } else {
                        body.push(f);
                    }
                }
                if self.match_tok(tk!("}")) {
                    break;
                }
            }
            let _ = self.match_tok(tk!("}"));
        }
        Some(ImplDecl {
            target,
            trait_name,
            body,
            line,
            attrs,
        })
    }

    fn parse_function(
        &mut self,
        is_public: bool,
        is_test: bool,
        attrs: Vec<Attribute>,
    ) -> Option<Function> {
        let name_tok = self.consume_ident("PAR001", "kazi haina jina")?;
        let line = name_tok.line;
        let _span = asili_trace::enter(&name_tok.lexeme, line as u32);
        let column = name_tok.column;
        self.consume(tk!("("), "PAR002", "kazi inahitaji '('")?;
        let params = self.parse_params();
        self.consume(tk!(")"), "PAR003", "kazi inahitaji ')' baada ya params")?;
        self.consume(
            tk!("->"),
            "PAR004",
            "kazi inahitaji aina ya kurudisha baada ya '->'",
        )?;
        let return_type = self.parse_type();
        let body = self.parse_body()?;

        Some(Function {
            name: name_tok.lexeme.clone(),
            params,
            return_type,
            body,
            is_test,
            is_public,
            line,
            column,
            attrs,
        })
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.check(tk!(")")) {
            return params;
        }

        while let Some(name) = self.consume_ident("PAR010", "hoja inahitaji jina") {
            if self
                .consume(tk!(":"), "PAR011", "hoja inahitaji ':'")
                .is_none()
            {
                break;
            }
            let ty = self.parse_type();
            params.push(Param {
                name: name.lexeme.clone(),
                ty,
                line: name.line,
                column: name.column,
            });

            if self.match_tok(tk!(",")) {
                if self.check(tk!(")")) {
                    break;
                }
                continue;
            }
            break;
        }

        params
    }

    fn parse_type(&mut self) -> TypeExpr {
        let mut name = String::new();
        let mut depth = 0usize;
        let mut last_line: Option<usize> = None;
        while !self.is_eof() {
            let t = self.peek();
            let l = t.lexeme.as_str();
            if depth == 0
                && matches!(
                    t.kind,
                    TokenKind::Comma
                        | TokenKind::RParen
                        | TokenKind::LBrace
                        | TokenKind::RBrace
                        | TokenKind::Assign
                        | TokenKind::Arrow
                        | TokenKind::KwKama
                )
            {
                break;
            }
            // A type name never legitimately spans a line break at depth 0 (outside `<...>`) in
            // this language's style — without this check, a bare `kama Type` cast immediately
            // followed by the next statement (e.g. `weka x = 1 kama Namba` then `weka y = ...`
            // on the next line) silently swallows that next statement's tokens into the type
            // name, since none of the stop-tokens above (",", ")", "{", "}", "=", "->") appear
            // between the type name and an unrelated following statement.
            if depth == 0 {
                if let Some(prev_line) = last_line {
                    if self.peek().line != prev_line {
                        break;
                    }
                }
            }
            if l == "<" {
                depth += 1;
            } else if l == ">" && depth > 0 {
                depth -= 1;
            } else if l == ">>" && depth > 0 {
                // The lexer reads `>>` as one (shift) token; in a type it closes two generics:
                // `Orodha<Orodha<Namba>>`.
                depth = depth.saturating_sub(2);
            }
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(l);
            last_line = Some(self.peek().line);
            self.pos += 1;
            if depth == 0 && self.check(tk!("{")) {
                break;
            }
        }
        TypeExpr {
            name: name.trim().to_string(),
        }
    }

    /// A declaration may introduce several bindings: `weka a = 1, b = 2`.
    /// Each binding is lowered to the ordinary single-binding AST form.
    fn parse_let_group(&mut self) -> Option<Vec<Stmt>> {
        let mutable = self.match_tok(tk!("weka"));
        if !mutable {
            self.consume(tk!("thabiti"), "PAR040", "weka/thabiti inahitaji jina")?;
        }
        let mut statements = Vec::new();
        loop {
            let stmt = if self.check(tk!("(")) {
                let line = self.peek().line;
                let pattern = self.parse_pattern()?;
                self.consume(tk!("="), "PAR040", "muundo wa weka unahitaji '='")?;
                let value = self.parse_expression()?;
                Stmt::LetPattern {
                    mutable,
                    pattern,
                    value,
                    line,
                }
            } else {
                self.parse_let_stmt(mutable)?
            };
            statements.push(stmt);
            if !self.match_tok(tk!(",")) {
                break;
            }
        }
        Some(statements)
    }

    /// A statement with no block of its own (the block machine handles `ikiwa`, `kwa`, `wakati`,
    /// `linganisha` and `weka` groups).
    fn parse_simple_stmt(&mut self) -> Option<Stmt> {
        if self.match_tok(tk!("vunja")) {
            let line = self.prev().line;
            let label = self.parse_optional_label("PAR049", "vunja lebo inahitaji jina");
            return Some(Stmt::Break { label, line });
        }
        if self.match_tok(tk!("endelea")) {
            let line = self.prev().line;
            let label = self.parse_optional_label("PAR044", "endelea lebo inahitaji jina");
            return Some(Stmt::Continue { label, line });
        }
        if self.match_tok(tk!("rejesha")) {
            let line = self.prev().line;
            if self.check(tk!("}")) {
                return Some(Stmt::Return { value: None, line });
            }
            let value = self.parse_expression()?;
            return Some(Stmt::Return {
                value: Some(value),
                line,
            });
        }
        if self.match_tok(tk!("tupa")) {
            let line = self.prev().line;
            let ident = self.consume_ident("PAR030", "tupa inahitaji jina")?;
            return Some(Stmt::Drop {
                name: ident.lexeme.clone(),
                line,
            });
        }

        // Index-assign: name[expr] = val  →  Stmt::Expr( name.ingiza(expr, val) )
        if self.check_ident() && self.check_n(1, tk!("[")) {
            // Scan forward past matching brackets to check for '=' after ']'
            let mut depth = 1usize;
            let mut j = self.pos + 2;
            while j < self.tokens.len() && depth > 0 {
                match self.tokens[j].kind {
                    TokenKind::LBracket => depth += 1,
                    TokenKind::RBracket => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    j += 1;
                }
            }
            let after = self.tokens.get(j + 1).map(|t| t.kind);
            if let (0, Some(op_tok)) = (depth, after.filter(|&k| compound_op(k).is_some())) {
                // `name[i] op= v`  →  `name.ingiza(i, name[i] op v)`.
                let (op_base, op) = compound_op(op_tok).expect("checked above");
                let name = self.advance().lexeme.clone();
                let line = self.prev().line;
                let column = self.prev().column;
                self.advance(); // consume [
                let idx = self.parse_expression()?;
                self.consume(tk!("]"), "PAR091", "fahirisi inahitaji ']'")?;
                let op_line = self.peek().line;
                let op_column = self.peek().column;
                let op_lexeme = self.advance().lexeme.clone();
                // The index is evaluated twice (read and write), so it must not call anything.
                if expr_has_call(&self.exprs, idx) {
                    self.errors.push(
                        Diagnostic::new(
                            "PAR096",
                            format!("fahirisi ya '{op_lexeme}' haiwezi kuwa na mwito wa kazi"),
                        )
                        .with_stage("uchanganuzi")
                        .with_span(op_line, op_column),
                    );
                    return None;
                }
                let val = self.parse_expression()?;
                let target = Expr::Ident { name, line, column };
                let base = self.exprs.add(target.clone());
                // The index is pure (checked above), so the read and the write share its node.
                let current = self.exprs.add(Expr::Index {
                    base,
                    index: idx,
                    line,
                });
                let receiver = self.exprs.add(target);
                let value = build_binary(&mut self.exprs, op_base, op, current, val, line);
                let expr = self.exprs.add(Expr::MethodCall {
                    receiver,
                    method_name: "ingiza".to_string(),
                    args: vec![idx, value],
                    line,
                });
                return Some(Stmt::Expr { expr, line });
            }
            if depth == 0 && after == Some(TokenKind::Assign) {
                let name = self.advance().lexeme.clone();
                let line = self.prev().line;
                let column = self.prev().column;
                self.advance(); // consume [
                let idx = self.parse_expression()?;
                self.consume(tk!("]"), "PAR091", "fahirisi inahitaji ']'")?;
                self.advance(); // consume =
                let val = self.parse_expression()?;
                let receiver = self.exprs.add(Expr::Ident { name, line, column });
                let expr = self.exprs.add(Expr::MethodCall {
                    receiver,
                    method_name: "ingiza".to_string(),
                    args: vec![idx, val],
                    line,
                });
                return Some(Stmt::Expr { expr, line });
            }
        }

        if self.check_ident() && self.check_n(1, tk!("=")) {
            let name = self.advance().lexeme.clone();
            let line = self.prev().line;
            let column = self.prev().column;
            self.advance();
            let expr = self.parse_expression()?;
            return Some(Stmt::Assign {
                name,
                op: AssignOp::Assign,
                value: expr,
                line,
                column,
            });
        }

        // `x %= e`, `x //= e`, `x &= e`, `x |= e`, `x ^= e`  →  `x = x op e`.
        if self.check_ident()
            && self.peek_n(1).is_some_and(|t| {
                matches!(
                    t.kind,
                    TokenKind::PercentEq
                        | TokenKind::SlashSlashEq
                        | TokenKind::AmpEq
                        | TokenKind::PipeEq
                        | TokenKind::CaretEq
                )
            })
        {
            let name = self.advance().lexeme.clone();
            let line = self.prev().line;
            let column = self.prev().column;
            let (op_base, op) = compound_op(self.advance().kind).expect("listed above");
            let rhs = self.parse_expression()?;
            let current = self.exprs.add(Expr::Ident {
                name: name.clone(),
                line,
                column,
            });
            return Some(Stmt::Assign {
                name,
                op: AssignOp::Assign,
                value: build_binary(&mut self.exprs, op_base, op, current, rhs, line),
                line,
                column,
            });
        }

        if self.check_ident()
            && self.peek_n(1).is_some_and(|t| {
                matches!(
                    t.kind,
                    TokenKind::PlusEq | TokenKind::MinusEq | TokenKind::StarEq | TokenKind::SlashEq
                )
            })
        {
            let name = self.advance().lexeme.clone();
            let line = self.prev().line;
            let column = self.prev().column;
            let op_tok = self.advance().lexeme.clone();
            let op = match op_tok.as_str() {
                "+=" => AssignOp::AddAssign,
                "-=" => AssignOp::SubAssign,
                "*=" => AssignOp::MulAssign,
                _ => AssignOp::DivAssign,
            };
            let expr = self.parse_expression()?;
            return Some(Stmt::Assign {
                name,
                op,
                value: expr,
                line,
                column,
            });
        }

        let line = self.peek().line;
        let expr = self.parse_expression()?;
        Some(Stmt::Expr { expr, line })
    }

    fn parse_let_stmt(&mut self, mutable: bool) -> Option<Stmt> {
        let name = self.consume_ident("PAR040", "weka/thabiti inahitaji jina")?;
        let mut ty = None;
        if self.match_tok(tk!(":")) {
            ty = Some(self.parse_type());
        }
        self.consume(tk!("="), "PAR041", "weka/thabiti inahitaji '='")?;
        let value = self.parse_expression()?;
        Some(Stmt::Let {
            mutable,
            name: name.lexeme.clone(),
            ty,
            value,
            line: name.line,
            column: name.column,
        })
    }

    /// Skip to the next thing that can start a top-level item, after an error, so one mistake
    /// is reported once rather than once per remaining token.
    fn skip_top_level(&mut self) {
        while !self.is_eof() {
            let t = self.peek();
            let item = matches!(
                t.kind,
                TokenKind::KwKazi
                    | TokenKind::KwUmma
                    | TokenKind::KwLeta
                    | TokenKind::KwUmbo
                    | TokenKind::KwJenum
                    | TokenKind::KwSifa
                    | TokenKind::KwShughuli
            ) || (t.column == 1
                && matches!(t.kind, TokenKind::KwThabiti | TokenKind::Hash));
            if item {
                break;
            }
            self.pos += 1;
        }
    }

    pub(crate) fn standard_enums(&self) -> Vec<EnumDecl> {
        vec![
            EnumDecl {
                name: "Chaguo".to_string(),
                generics: vec!["T".to_string()],
                variants: vec![
                    EnumVariant {
                        name: "Kuna".to_string(),
                        data: Some(TypeExpr {
                            name: "T".to_string(),
                        }),
                        line: 0,
                        column: 0,
                    },
                    EnumVariant {
                        name: "Hamna".to_string(),
                        data: None,
                        line: 0,
                        column: 0,
                    },
                ],
                line: 0,
                column: 0,
                is_public: true,
                attrs: Vec::new(),
            },
            EnumDecl {
                name: "Tokeo".to_string(),
                generics: vec!["T".to_string(), "E".to_string()],
                variants: vec![
                    EnumVariant {
                        name: "Sawa".to_string(),
                        data: Some(TypeExpr {
                            name: "T".to_string(),
                        }),
                        line: 0,
                        column: 0,
                    },
                    EnumVariant {
                        name: "Kosa".to_string(),
                        data: Some(TypeExpr {
                            name: "E".to_string(),
                        }),
                        line: 0,
                        column: 0,
                    },
                ],
                line: 0,
                column: 0,
                is_public: true,
                attrs: Vec::new(),
            },
        ]
    }

    /// Built-in traits seeded into every module, the same way `standard_enums()` seeds
    /// `Chaguo`/`Tokeo`. Not defined in an `.asi` file: `lib/std`'s `.asi` loader
    /// (`pata/cli/src/pipeline/interface_registry.rs`) is a line-by-line text parser, not the
    /// real lexer/parser, and cannot reliably parse a multi-line `sifa { ... }` body.
    pub(crate) fn standard_traits(&self) -> Vec<TraitDecl> {
        vec![
            TraitDecl {
                name: "Inasomeka".to_string(),
                methods: vec![TraitMethodSig {
                    name: "soma".to_string(),
                    params: vec![Param {
                        name: "self".to_string(),
                        ty: TypeExpr {
                            name: "Self".to_string(),
                        },
                        line: 0,
                        column: 0,
                    }],
                    return_type: TypeExpr {
                        name: "Tokeo<Neno, Neno>".to_string(),
                    },
                    line: 0,
                }],
                line: 0,
                column: 0,
                is_public: true,
                attrs: Vec::new(),
            },
            TraitDecl {
                name: "Inandikika".to_string(),
                methods: vec![TraitMethodSig {
                    name: "andika".to_string(),
                    params: vec![
                        Param {
                            name: "self".to_string(),
                            ty: TypeExpr {
                                name: "Self".to_string(),
                            },
                            line: 0,
                            column: 0,
                        },
                        Param {
                            name: "data".to_string(),
                            ty: TypeExpr {
                                name: "Neno".to_string(),
                            },
                            line: 0,
                            column: 0,
                        },
                    ],
                    return_type: TypeExpr {
                        name: "Tokeo<Tupu, Neno>".to_string(),
                    },
                    line: 0,
                }],
                line: 0,
                column: 0,
                is_public: true,
                attrs: Vec::new(),
            },
        ]
    }
}

/// The operator token and binary operator of a compound-assignment token (`+=` → `+`, ...).
fn compound_op(token: TokenKind) -> Option<(TokenKind, BinaryOp)> {
    Some(match token {
        TokenKind::PlusEq => (tk!("+"), BinaryOp::Add),
        TokenKind::MinusEq => (tk!("-"), BinaryOp::Sub),
        TokenKind::StarEq => (tk!("*"), BinaryOp::Mul),
        TokenKind::SlashEq => (tk!("/"), BinaryOp::Div),
        TokenKind::SlashSlashEq => (tk!("//"), BinaryOp::Div),
        TokenKind::PercentEq => (tk!("%"), BinaryOp::Rem),
        TokenKind::AmpEq => (tk!("&"), BinaryOp::BitAnd),
        TokenKind::PipeEq => (tk!("|"), BinaryOp::BitOr),
        TokenKind::CaretEq => (tk!("^"), BinaryOp::BitXor),
        _ => return None,
    })
}

/// `left <tok> right`, added to `exprs`. Every binary expression, including compound
/// assignments, is built here: floor division `a // b` is `sakafu(a / b)`, so it shares
/// `sakafu`'s semantics (and the native backend's integer-division lowering) instead of being a
/// second implementation.
fn build_binary(
    exprs: &mut Exprs,
    tok: TokenKind,
    op: BinaryOp,
    left: ExprId,
    right: ExprId,
    line: usize,
) -> ExprId {
    let quotient = exprs.add(Expr::Binary {
        left,
        op,
        right,
        line,
    });
    if tok != TokenKind::SlashSlash {
        return quotient;
    }
    let callee = exprs.add(Expr::Ident {
        name: "sakafu".to_string(),
        line,
        column: 0,
    });
    exprs.add(Expr::Call {
        callee,
        args: vec![quotient],
        line,
    })
}

/// Whether evaluating `expr` could call a `kazi`, builtin or method (an explicit worklist, no
/// recursion).
fn expr_has_call(exprs: &Exprs, expr: ExprId) -> bool {
    let mut work = vec![expr];
    while let Some(e) = work.pop() {
        match &exprs[e] {
            // Pure numeric builtins may be evaluated twice without any observable difference
            // (builtins take precedence over a same-named `kazi`).
            Expr::Call { callee, args, .. }
                if matches!(&exprs[*callee], Expr::Ident { name, .. }
                    if matches!(name.as_str(), "sakafu" | "dari" | "abs" | "mzizi")) =>
            {
                work.extend(args);
            }
            Expr::Call { .. } | Expr::MethodCall { .. } => return true,
            other => work.extend(other.children()),
        }
    }
    false
}
