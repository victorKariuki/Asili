//! Parsing: module, statements, expressions, patterns.

use asili_diagnostics::Diagnostic;
use std::mem;

use crate::{
    AssignOp, Attribute, BinaryOp, Block, Constant, EnumDecl, EnumVariant, Expr, ForMode,
    Function, Import, ImportPath, ImplDecl, MatchArm, Module, Param, Pattern, Stmt, StructDecl,
    TraitDecl, TypeExpr, UnaryOp,
};
use crate::cursor::Parser;

const MAX_RECURSION_DEPTH: usize = 1000;

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
            if self.check("#") && self.check_n(1, "[") {
                if let Some(attr) = self.parse_attribute() {
                    pending_test = attr.name == "jaribio";
                    pending_attrs.push(attr);
                } else {
                    self.pos += 1;
                }
                continue;
            }

            if self.match_tok("leta") {
                let line = self.prev().line;
                if let Some(path) = self.parse_import_path() {
                    imports.push(Import { path, line });
                }
                continue;
            }

            if self.match_tok("thabiti") {
                let line = self.prev().line;
                if let Some(const_decl) = self.parse_module_constant(line) {
                    constants.push(const_decl);
                }
                continue;
            }

            let is_public = self.match_tok("umma");
            if self.match_tok("kazi") {
                if let Some(func) =
                    self.parse_function(is_public, pending_test, mem::take(&mut pending_attrs))
                {
                    functions.push(func);
                }
                pending_test = false;
                continue;
            }

            if self.match_tok("jenum") {
                if let Some(decl) = self.parse_enum_decl(is_public, mem::take(&mut pending_attrs)) {
                    enums.push(decl);
                }
                continue;
            }

            if self.match_tok("umbo") {
                if let Some(decl) = self.parse_struct_decl(is_public, mem::take(&mut pending_attrs)) {
                    structs.push(decl);
                }
                continue;
            }

            if self.match_tok("sifa") {
                if let Some(decl) = self.parse_trait_decl(is_public, mem::take(&mut pending_attrs)) {
                    traits.push(decl);
                }
                continue;
            }

            if self.match_tok("shughuli") {
                if self.match_tok("ya") {
                    if let Some(decl) = self.parse_impl_decl(mem::take(&mut pending_attrs)) {
                        impls.push(decl);
                    }
                } else {
                    self.err_here("PAR901", "shughuli inahitaji 'ya'");
                    self.skip_top_level();
                }
                continue;
            }

            self.err_here("PAR000", "token isiyotegemewa kwenye kiwango cha juu");
            self.pos += 1;
        }

        enums.extend(self.standard_enums());

        Module {
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
        let name = name_tok.lexeme;
        let column = name_tok.column;
        // Explicit type annotation is optional (`thabiti X = 1` vs `thabiti X: Namba = 1`), but
        // the leading ':' must actually be consumed when present — parse_type() has no special
        // handling for a leading ':' itself (":" isn't one of its stop-tokens), so without this
        // `match_tok`, a typed constant like `thabiti PI: Namba = 3.14` silently parsed its type
        // as the bogus string ": Namba" instead of "Namba", which no ValueType recognizes, so it
        // always resolved to ValueType::Unknown regardless of what was actually written.
        let ty = if self.match_tok(":") {
            self.parse_type()
        } else {
            TypeExpr { name: String::new() }
        };
        self.consume("=", "PAR041", "thabiti inahitaji '='")?;
        let value = self.parse_expression()?;
        Some(Constant { name, ty, value, line, column })
    }

    fn parse_import_path(&mut self) -> Option<ImportPath> {
        let module = self.consume_ident("PAR059", "leta inahitaji jina la moduli")?.lexeme;
        if self.match_tok("::") && self.match_tok("{") {
            let mut names = Vec::new();
            loop {
                let t = self.consume_ident("PAR061", "leta ya kuchagua inahitaji jina")?;
                names.push(t.lexeme);
                if self.match_tok("}") {
                    break;
                }
                self.consume(",", "PAR062", "leta ya kuchagua inahitaji ',' kati ya majina")?;
            }
            return Some(ImportPath::Selective {
                module,
                names,
            });
        }
        Some(ImportPath::Full(module))
    }

    fn parse_attribute(&mut self) -> Option<Attribute> {
        self.consume("#", "PAR080", "kiambatanisho inahitaji '#'")?;
        self.consume("[", "PAR081", "kiambatanisho inahitaji '['")?;
        let name = self.consume_ident("PAR082", "kiambatanisho inahitaji jina")?;
        let mut args = None;
        if self.match_tok("(") {
            let mut raw = String::new();
            while !self.is_eof() && !self.check(")") {
                if !raw.is_empty() {
                    raw.push(' ');
                }
                raw.push_str(&self.advance().lexeme);
            }
            self.consume(")", "PAR083", "kiambatanisho hoja inahitaji ')'")?;
            args = Some(raw);
        }
        self.consume("]", "PAR084", "kiambatanisho inahitaji ']'")?;
        Some(Attribute {
            name: name.lexeme,
            args,
            line: name.line,
        })
    }

    fn parse_generic_names(&mut self) -> Vec<String> {
        let mut gens = Vec::new();
        if !self.match_tok("<") {
            return gens;
        }
        loop {
            if let Some(id) = self.consume_ident("PAR085", "jumla inahitaji jina") {
                gens.push(id.lexeme);
            }
            if self.match_tok(",") {
                continue;
            }
            break;
        }
        let _ = self.consume(">", "PAR086", "orodha ya jumla inahitaji '>'");
        gens
    }

    fn skip_body(&mut self) {
        if !self.match_tok("{") {
            return;
        }
        let mut depth = 1usize;
        while !self.is_eof() && depth > 0 {
            if self.match_tok("{") {
                depth += 1;
            } else if self.match_tok("}") {
                depth -= 1;
            } else {
                self.pos += 1;
            }
        }
    }

    fn parse_struct_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<StructDecl> {
        let name = self.consume_ident("PAR902", "umbo inahitaji jina")?;
        let line = name.line;
        let column = name.column;
        let generics = self.parse_generic_names();
        let fields = if self.match_tok("{") {
            let mut flds = Vec::new();
            loop {
                if self.match_tok("}") {
                    break;
                }
                let field_name = self.consume_ident("PAR902", "umbo inahitaji jina la uga")?;
                let ty = if self.match_tok(":") {
                    Some(self.parse_type())
                } else {
                    None
                };
                flds.push((field_name.lexeme, ty));
                if !self.match_tok(",") {
                    // Fields may be newline-separated instead of comma-separated (e.g.
                    // `umbo P { x: Namba\n y: Namba }`) — only treat it as the end of the
                    // field list if '}' actually follows; otherwise loop for the next field.
                    if self.match_tok("}") {
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
            name: name.lexeme,
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
        let variants = if self.match_tok("{") {
            let mut vars = Vec::new();
            loop {
                if self.match_tok("}") {
                    break;
                }
                let var_name = self.consume_ident("PAR905", "jenum inahitaji jina la kigezo")?;
                let var_line = var_name.line;
                let var_column = var_name.column;
                let data = if self.match_tok("(") {
                    let ty = self.parse_type();
                    self.consume(")", "PAR905", "kigezo inahitaji ')'")?;
                    Some(ty)
                } else {
                    None
                };
                vars.push(EnumVariant {
                    name: var_name.lexeme,
                    data,
                    line: var_line,
                    column: var_column,
                });
                if !self.match_tok(",") {
                    let _ = self.consume("}", "PAR905", "jenum inahitaji '}'");
                    break;
                }
            }
            vars
        } else {
            Vec::new()
        };
        Some(EnumDecl {
            name: name.lexeme,
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
        self.skip_body();
        Some(TraitDecl {
            name: name.lexeme,
            line,
            column,
            attrs,
            is_public,
        })
    }

    fn parse_impl_decl(&mut self, attrs: Vec<Attribute>) -> Option<ImplDecl> {
        let first = self.consume_ident("PAR904", "shughuli ya inahitaji jina la aina")?;
        let line = first.line;
        let mut trait_name = None;
        let mut target = first.lexeme.clone();
        if self.match_tok("kwa") {
            // "shughuli ya Trait kwa Target { }" — kwa keyword syntax
            trait_name = Some(first.lexeme);
            if let Some(t) = self.consume_ident("PAR905", "shughuli ya kwa inahitaji jina la aina") {
                target = t.lexeme;
            }
        } else if self.match_tok(":") {
            // "shughuli ya Target: Trait { }" — colon syntax
            if let Some(t) = self.consume_ident("PAR905", "shughuli ya : inahitaji jina la sifa") {
                trait_name = Some(t.lexeme);
            }
        }
        let mut body = Vec::new();
        if self.match_tok("{") {
            while self.match_tok("kazi") {
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
                if self.match_tok("}") {
                    break;
                }
            }
            let _ = self.match_tok("}");
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
        let column = name_tok.column;
        self.consume("(", "PAR002", "kazi inahitaji '('")?;
        let params = self.parse_params();
        self.consume(")", "PAR003", "kazi inahitaji ')' baada ya params")?;
        self.consume("->", "PAR004", "kazi inahitaji aina ya kurudisha baada ya '->'")?;
        let return_type = self.parse_type();
        let body = self.parse_block()?;

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
        if self.check(")") {
            return params;
        }

        while let Some(name) = self.consume_ident("PAR010", "hoja inahitaji jina") {
            if self.consume(":", "PAR011", "hoja inahitaji ':'").is_none() {
                break;
            }
            let ty = self.parse_type();
            params.push(Param {
                name: name.lexeme,
                ty,
                line: name.line,
                column: name.column,
            });

            if self.match_tok(",") {
                if self.check(")") {
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
            let l = self.peek().lexeme.as_str();
            if depth == 0 && [",", ")", "{", "}", "=", "->", "kama"].contains(&l) {
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
            }
            if !name.is_empty() {
                name.push(' ');
            }
            name.push_str(l);
            last_line = Some(self.peek().line);
            self.pos += 1;
            if depth == 0 && self.check("{") {
                break;
            }
        }
        TypeExpr {
            name: name.trim().to_string(),
        }
    }

    fn parse_block(&mut self) -> Option<Block> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            let mut d = Diagnostic::new("PAR073", "undani mno").with_stage("uchanganuzi");
            if !self.is_eof() {
                d = d.with_span(self.peek().line, self.peek().column);
            }
            self.errors.push(d);
            self.depth -= 1;
            return None;
        }
        let result = stacker::maybe_grow(32 * 1024, 1024 * 1024, || self.parse_block_inner());
        self.depth -= 1;
        result
    }

    fn parse_block_inner(&mut self) -> Option<Block> {
        self.consume("{", "PAR020", "kizuizi inahitaji '{'")?;
        let mut statements = Vec::new();
        while !self.is_eof() && !self.check("}") {
            if let Some(stmt) = self.parse_stmt() {
                statements.push(stmt);
            } else {
                self.pos += 1;
            }
        }
        self.consume("}", "PAR021", "kizuizi inahitaji '}'")?;
        Some(Block { statements })
    }

    fn parse_stmt(&mut self) -> Option<Stmt> {
        let loop_label = if self.match_tok("lebo") {
            let id = self
                .consume_ident("PAR042", "lebo inahitaji jina")
                .map(|t| t.lexeme.trim_start_matches('\'').to_string());
            let _ = self.consume(":", "PAR043", "lebo inahitaji ':'");
            id
        } else {
            None
        };
        if self.match_tok("weka") || self.match_tok("thabiti") {
            let mutable = self.prev().lexeme == "weka";
            return self.parse_let_stmt(mutable);
        }
        if self.match_tok("ikiwa") {
            return self.parse_if_stmt();
        }
        if self.match_tok("kwa") {
            return self.parse_for_stmt(loop_label);
        }
        if self.match_tok("wakati") {
            return self.parse_while_stmt(loop_label);
        }
        if self.match_tok("linganisha") {
            return self.parse_match_stmt();
        }
        if self.match_tok("vunja") {
            let line = self.prev().line;
            let label = self.parse_optional_label("PAR049", "vunja lebo inahitaji jina");
            return Some(Stmt::Break { label, line });
        }
        if self.match_tok("endelea") {
            let line = self.prev().line;
            let label = self.parse_optional_label("PAR044", "endelea lebo inahitaji jina");
            return Some(Stmt::Continue { label, line });
        }
        if self.match_tok("rejesha") {
            let line = self.prev().line;
            if self.check("}") {
                return Some(Stmt::Return { value: None, line });
            }
            let value = self.parse_expression()?;
            return Some(Stmt::Return {
                value: Some(value),
                line,
            });
        }
        if self.match_tok("tupa") {
            let line = self.prev().line;
            let ident = self.consume_ident("PAR030", "tupa inahitaji jina")?;
            return Some(Stmt::Drop {
                name: ident.lexeme,
                line,
            });
        }

        // Index-assign: name[expr] = val  →  Stmt::Expr( name.ingiza(expr, val) )
        if self.check_ident() && self.check_n(1, "[") {
            // Scan forward past matching brackets to check for '=' after ']'
            let mut depth = 1usize;
            let mut j = self.pos + 2;
            while j < self.tokens.len() && depth > 0 {
                match self.tokens[j].lexeme.as_str() {
                    "[" => depth += 1,
                    "]" => depth -= 1,
                    _ => {}
                }
                if depth > 0 { j += 1; }
            }
            if depth == 0 && self.tokens.get(j + 1).map(|t| t.lexeme.as_str()) == Some("=") {
                let name = self.advance().lexeme.clone();
                let line = self.prev().line;
                let column = self.prev().column;
                self.advance(); // consume [
                let idx = self.parse_expression()?;
                self.consume("]", "PAR091", "fahirisi inahitaji ']'")?;
                self.advance(); // consume =
                let val = self.parse_expression()?;
                return Some(Stmt::Expr {
                    expr: Expr::MethodCall {
                        receiver: Box::new(Expr::Ident { name, line, column }),
                        method_name: "ingiza".to_string(),
                        args: vec![idx, val],
                        line,
                    },
                    line,
                });
            }
        }

        if self.check_ident() && self.check_n(1, "=") {
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

        if self.check_ident() && ["+=", "-=", "*=", "/="].contains(&self.peek_n(1).map(|t| t.lexeme.as_str()).unwrap_or("")) {
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

    fn parse_for_stmt(&mut self, label: Option<String>) -> Option<Stmt> {
        let line = self.prev().line;
        let var_tok = self.consume_ident("PAR054", "kwa inahitaji jina")?;
        let var = var_tok.lexeme;
        let var_column = var_tok.column;
        if self.match_tok("katika") {
            let expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Some(Stmt::For {
                label,
                var,
                var_column,
                mode: ForMode::InExpr(expr),
                body,
                line,
            });
        }
        if self.match_tok("kutoka") {
            let start = self.parse_expression()?;
            self.consume("hadi", "PAR055", "kwa kutoka inahitaji 'hadi'")?;
            let end = self.parse_expression()?;
            let body = self.parse_block()?;
            return Some(Stmt::For {
                label,
                var,
                var_column,
                mode: ForMode::Range { start, end },
                body,
                line,
            });
        }
        self.err_here("PAR056", "kwa inahitaji 'katika' au 'kutoka ... hadi ...'");
        None
    }

    fn parse_let_stmt(&mut self, mutable: bool) -> Option<Stmt> {
        let name = self.consume_ident("PAR040", "weka/thabiti inahitaji jina")?;
        let mut ty = None;
        if self.match_tok(":") {
            ty = Some(self.parse_type());
        }
        self.consume("=", "PAR041", "weka/thabiti inahitaji '='")?;
        let value = self.parse_expression()?;
        Some(Stmt::Let {
            mutable,
            name: name.lexeme,
            ty,
            value,
            line: name.line,
            column: name.column,
        })
    }

    fn parse_if_stmt(&mut self) -> Option<Stmt> {
        let line = self.prev().line;
        let cond = self.parse_expression()?;
        let then_block = self.parse_block()?;

        let mut else_if = Vec::new();
        while self.match_tok("au_ikiwa") {
            let c = self.parse_expression()?;
            let b = self.parse_block()?;
            else_if.push((c, b));
        }

        let else_block = if self.match_tok("vinginevyo") {
            Some(self.parse_block()?)
        } else {
            None
        };

        Some(Stmt::If {
            cond,
            then_block,
            else_if,
            else_block,
            line,
        })
    }

    fn parse_while_stmt(&mut self, label: Option<String>) -> Option<Stmt> {
        let line = self.prev().line;
        let cond = if self.match_tok("milele") {
            Expr::Bool(true)
        } else {
            self.parse_expression()?
        };
        let body = self.parse_block()?;
        Some(Stmt::While {
            label,
            cond,
            body,
            line,
        })
    }

    fn parse_match_stmt(&mut self) -> Option<Stmt> {
        let line = self.prev().line;
        let expr = self.parse_expression()?;
        self.consume("{", "PAR050", "linganisha inahitaji '{'")?;
        let mut arms = Vec::new();
        while !self.is_eof() && !self.check("}") {
            let pat = self.parse_pattern()?;
            self.consume("=>", "PAR051", "mkono wa linganisha unahitaji '=>'")?;
            let body = self.parse_block()?;
            arms.push(MatchArm {
                pattern: pat,
                body,
                line: self.prev().line,
            });
            self.match_tok(",");
        }
        self.consume("}", "PAR052", "linganisha inahitaji '}'")?;
        Some(Stmt::Match { expr, arms, line })
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        if self.match_tok("(") {
            let p1 = self.parse_pattern()?;
            if self.match_tok(",") {
                let p2 = self.parse_pattern()?;
                self.consume(")", "PAR053", "muundo wa jozi unahitaji ')'")?;
                return Some(Pattern::Jozi(Box::new(p1), Box::new(p2)));
            }
            self.consume(")", "PAR053", "muundo unahitaji ')'")?;
            return Some(p1);
        }
        if self.match_tok("_") {
            return Some(Pattern::Wildcard);
        }
        if self.peek().lexeme.starts_with('"') {
            let t = self.advance();
            return Some(Pattern::Literal(Expr::String(strip_string_lexeme_quotes(&t.lexeme))));
        }
        if self.peek().lexeme.starts_with("CHAR:") {
            let t = self.advance();
            let ch = t.lexeme.strip_prefix("CHAR:").and_then(|s| s.chars().next()).unwrap_or('\0');
            return Some(Pattern::Literal(Expr::Char(ch)));
        }
        if self.peek().lexeme.chars().all(|c| c.is_ascii_digit() || c == '.') {
            let t = self.advance();
            return Some(Pattern::Literal(Expr::Number(t.lexeme.clone())));
        }
        if self.match_tok("Hamna") {
            return Some(Pattern::Literal(Expr::Hamna));
        }
        if self.match_tok("kweli") {
            return Some(Pattern::Literal(Expr::Bool(true)));
        }
        if self.match_tok("si_kweli") {
            return Some(Pattern::Literal(Expr::Bool(false)));
        }
        if self.check_ident() {
            let t = self.advance();
            let name = t.lexeme.clone();
            let line = t.line;
            let column = t.column;
            if self.match_tok("::") {
                let variant_tok = self.consume_ident("PAR085", "jenum pattern inahitaji jina la kigezo")?;
                let variant_name = variant_tok.lexeme.clone();
                let variant_line = variant_tok.line;
                let variant_column = variant_tok.column;
                let data = if self.match_tok("(") {
                    let sub = self.parse_pattern()?;
                    self.consume(")", "PAR086", "jenum pattern inahitaji ')'")?;
                    Some(Box::new(sub))
                } else {
                    None
                };
                return Some(Pattern::Enum {
                    enum_name: name,
                    variant_name,
                    data,
                    variant_line,
                    variant_column,
                });
            }
            if self.match_tok("{") {
                let mut fields = Vec::new();
                loop {
                    if self.match_tok("}") {
                        break;
                    }
                    let fname = self.consume_ident("PAR053", "umbo pattern inahitaji jina la uga")?;
                    self.consume(":", "PAR053", "umbo pattern inahitaji ':'")?;
                    let sub = self.parse_pattern()?;
                    fields.push((fname.lexeme, sub));
                    if !self.match_tok(",") {
                        self.consume("}", "PAR053", "umbo pattern inahitaji '}'")?;
                        break;
                    }
                }
                return Some(Pattern::Struct {
                    struct_name: name,
                    line,
                    column,
                    fields,
                });
            }
            return Some(Pattern::Ident { name, line, column });
        }
        self.err_here("PAR053", "muundo wa linganisha haueleweki — inahitaji thamani, jina, jozi, jenum, au umbo");
        None
    }

    fn parse_expression(&mut self) -> Option<Expr> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            if !self.is_eof() {
                self.errors.push(
                    Diagnostic::new("PAR073", "undani mno")
                        .with_stage("uchanganuzi")
                        .with_span(self.peek().line, self.peek().column),
                );
            }
            self.depth -= 1;
            return None;
        }
        let result = stacker::maybe_grow(32 * 1024, 1024 * 1024, || self.parse_or());
        self.depth -= 1;
        result
    }

    /// Left-associative binary level: parse `left`, then repeatedly match (token, op) and `next` for right.
    fn parse_binary_left<F>(&mut self, mut left: Expr, pairs: &[(&str, BinaryOp)], next: F) -> Option<Expr>
    where
        F: Fn(&mut Self) -> Option<Expr>,
    {
        loop {
            let mut matched = false;
            for (tok, op) in pairs {
                if self.match_tok(tok) {
                    matched = true;
                    let line = self.prev().line;
                    let right = next(self)?;
                    left = Expr::Binary {
                        left: Box::new(left),
                        op: op.clone(),
                        right: Box::new(right),
                        line,
                    };
                    break;
                }
            }
            if !matched {
                break;
            }
        }
        Some(left)
    }

    fn parse_or(&mut self) -> Option<Expr> {
        let left = self.parse_and()?;
        self.parse_binary_left(left, &[("au", BinaryOp::Or), ("||", BinaryOp::Or)], |p| p.parse_and())
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let left = self.parse_bitwise_or()?;
        self.parse_binary_left(left, &[("na", BinaryOp::And), ("&&", BinaryOp::And)], |p| p.parse_bitwise_or())
    }

    fn parse_bitwise_or(&mut self) -> Option<Expr> {
        let left = self.parse_bitwise_xor()?;
        self.parse_binary_left(left, &[("au_biti", BinaryOp::BitOr), ("|", BinaryOp::BitOr)], |p| p.parse_bitwise_xor())
    }

    fn parse_bitwise_xor(&mut self) -> Option<Expr> {
        let left = self.parse_bitwise_and()?;
        self.parse_binary_left(left, &[("xor_biti", BinaryOp::BitXor)], |p| p.parse_bitwise_and())
    }

    fn parse_bitwise_and(&mut self) -> Option<Expr> {
        let left = self.parse_equality()?;
        self.parse_binary_left(left, &[("na_biti", BinaryOp::BitAnd)], |p| p.parse_equality())
    }

    fn parse_equality(&mut self) -> Option<Expr> {
        let left = self.parse_comparison()?;
        self.parse_binary_left(
            left,
            &[("==", BinaryOp::Eq), ("!=", BinaryOp::Ne)],
            |p| p.parse_comparison(),
        )
    }

    fn parse_comparison(&mut self) -> Option<Expr> {
        let left = self.parse_shift()?;
        self.parse_binary_left(
            left,
            &[
                (">=", BinaryOp::Ge),
                ("<=", BinaryOp::Le),
                (">", BinaryOp::Gt),
                ("<", BinaryOp::Lt),
            ],
            |p| p.parse_shift(),
        )
    }

    fn parse_shift(&mut self) -> Option<Expr> {
        let left = self.parse_term()?;
        self.parse_binary_left(
            left,
            &[
                ("sogeza_kushoto", BinaryOp::Shl),
                ("sogeza_kulia", BinaryOp::Shr),
            ],
            |p| p.parse_term(),
        )
    }

    fn parse_term(&mut self) -> Option<Expr> {
        let left = self.parse_factor()?;
        self.parse_binary_left(
            left,
            &[("+", BinaryOp::Add), ("-", BinaryOp::Sub)],
            |p| p.parse_factor(),
        )
    }

    fn parse_factor(&mut self) -> Option<Expr> {
        let left = self.parse_power()?;
        self.parse_binary_left(
            left,
            &[
                ("*", BinaryOp::Mul),
                ("/", BinaryOp::Div),
                ("%", BinaryOp::Rem),
            ],
            |p| p.parse_power(),
        )
    }

    fn parse_power(&mut self) -> Option<Expr> {
        let left = self.parse_cast()?;
        self.parse_binary_left(left, &[("**", BinaryOp::Pow)], |p| p.parse_cast())
    }

    fn parse_cast(&mut self) -> Option<Expr> {
        let mut expr = self.parse_unary()?;
        while self.match_tok("kama") {
            let line = self.prev().line;
            let ty = self.parse_type();
            expr = Expr::Cast {
                expr: Box::new(expr),
                ty,
                line,
            };
        }
        Some(expr)
    }

    fn parse_unary(&mut self) -> Option<Expr> {
        const UNARY_OPS: &[(&str, UnaryOp)] = &[
            ("-", UnaryOp::Neg),
            ("siyo", UnaryOp::Not),
            ("!", UnaryOp::Not),
            ("siyo_biti", UnaryOp::BitNot),
            ("azima", UnaryOp::BorrowImm),
            ("azima_tenda", UnaryOp::BorrowMut),
            ("jaribu", UnaryOp::Jaribu),
        ];
        for (tok, op) in UNARY_OPS {
            if self.match_tok(tok) {
                let line = self.prev().line;
                let right = self.parse_unary()?;
                return Some(Expr::Unary {
                    op: op.clone(),
                    expr: Box::new(right),
                    line,
                });
            }
        }
        self.parse_postfix()
    }

    /// Parse comma-separated expressions until ")", then consume ")". Call after consuming "(".
    fn parse_paren_args(
        &mut self,
        close_code: &'static str,
        close_msg: &str,
    ) -> Option<Vec<Expr>> {
        let mut args = Vec::new();
        while !self.check(")") {
            args.push(self.parse_expression()?);
            if !self.match_tok(",") {
                break;
            }
        }
        self.consume(")", close_code, close_msg)?;
        Some(args)
    }

    fn parse_postfix(&mut self) -> Option<Expr> {
        let mut expr = self.parse_primary()?;
        loop {
            if self.match_tok("(") {
                let args = self.parse_paren_args("PAR060", "mwito wa kazi unahitaji ')'")?;
                let line = self.prev().line;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    line,
                };
                continue;
            }
            if self.match_tok(".") {
                let name_tok = self.consume_ident("PAR063", "uga au njia unahitaji jina")?;
                let name = name_tok.lexeme;
                let line = name_tok.line;
                let field_column = name_tok.column;
                if self.match_tok("(") {
                    let args = self.parse_paren_args("PAR065", "mwito wa njia unahitaji ')'")?;
                    expr = Expr::MethodCall {
                        receiver: Box::new(expr),
                        method_name: name,
                        args,
                        line,
                    };
                } else {
                    expr = Expr::FieldAccess {
                        receiver: Box::new(expr),
                        field: name,
                        line,
                        field_line: line,
                        field_column,
                    };
                }
                continue;
            }
            if self.match_tok("[") {
                let idx = self.parse_expression()?;
                self.consume("]", "PAR079", "fahirisi inahitaji ']'")?;
                let line = self.prev().line;
                expr = Expr::Index {
                    base: Box::new(expr),
                    index: Box::new(idx),
                    line,
                };
                continue;
            }
            if self.match_tok("?") {
                let line = self.prev().line;
                expr = Expr::Propagate {
                    expr: Box::new(expr),
                    line,
                };
                continue;
            }
            if self.match_tok("::") {
                if let Expr::Ident { name: enum_name, .. } = expr {
                    let variant_tok = self.consume_ident("PAR080", "jenum kigezo inahitaji jina")?;
                    let variant_name = variant_tok.lexeme;
                    let line = variant_tok.line;
                    let column = variant_tok.column;
                    let data = if self.match_tok("(") {
                        let d = self.parse_expression()?;
                        self.consume(")", "PAR081", "jenum kigezo data inahitaji ')'")?;
                        Some(Box::new(d))
                    } else {
                        None
                    };
                    expr = Expr::EnumConstruct {
                        enum_name,
                        variant_name,
                        data,
                        line,
                        column,
                    };
                } else {
                    self.err_here("PAR082", ":: inahitaji jina la jenum");
                    return None;
                }
                continue;
            }
            break;
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expr> {
        if self.match_tok("(") {
            let expr = self.parse_expression()?;
            self.consume(")", "PAR070", "kikundi kinahitaji ')'")?;
            return Some(Expr::Group(Box::new(expr)));
        }

        if self.match_tok("kweli") {
            return Some(Expr::Bool(true));
        }
        if self.match_tok("si_kweli") {
            return Some(Expr::Bool(false));
        }
        if self.match_tok("Hamna") {
            return Some(Expr::Hamna);
        }

        if self.match_tok("[") {
            let mut elements = Vec::new();
            if !self.match_tok("]") {
                loop {
                    elements.push(self.parse_expression()?);
                    if !self.match_tok(",") {
                        break;
                    }
                }
                self.consume("]", "PAR079", "orodha inahitaji ']'")?;
            }
            return Some(Expr::List {
                elements,
                line: self.prev().line,
            });
        }

        if self.match_tok("{") {
            let mut entries = Vec::new();
            if !self.match_tok("}") {
                loop {
                    let key = self.parse_expression()?;
                    self.consume(":", "PAR053", "kamusi inahitaji ':'")?;
                    let val = self.parse_expression()?;
                    entries.push((key, val));
                    if !self.match_tok(",") {
                        break;
                    }
                }
                self.consume("}", "PAR053", "kamusi inahitaji '}'")?;
            }
            return Some(Expr::Map {
                entries,
                line: self.prev().line,
            });
        }

        if self.peek().lexeme.starts_with("CHAR:") {
            let t = self.advance();
            let ch = t.lexeme.strip_prefix("CHAR:").and_then(|s| s.chars().next()).unwrap_or('\0');
            return Some(Expr::Char(ch));
        }

        if self.peek().lexeme.starts_with('"') {
            let t = self.advance();
            return Some(Expr::String(strip_string_lexeme_quotes(&t.lexeme)));
        }

        let next_lex = self.peek().lexeme.clone();
        let next_line = self.peek().line;
        let next_col = self.peek().column;

        if (next_lex.starts_with("0x") || next_lex.starts_with("0X")) && next_lex.len() > 2 {
            self.advance();
            self.errors.push(
                Diagnostic::new("PAR072", "heksadesimali (0x) haitumiki — tumia namba za desimali pekee")
                    .with_stage("uchanganuzi")
                    .with_span(next_line, next_col),
            );
            return None;
        }
        if (next_lex.starts_with("0b") || next_lex.starts_with("0B")) && next_lex.len() > 2 {
            self.advance();
            self.errors.push(
                Diagnostic::new("PAR072", "binari (0b) haitumiki — tumia namba za desimali pekee")
                    .with_stage("uchanganuzi")
                    .with_span(next_line, next_col),
            );
            return None;
        }

        if next_lex.chars().all(|c| c.is_ascii_digit() || c == '.') {
            let t = self.advance();
            let line = t.line;
            let column = t.column;
            let lexeme = t.lexeme.clone();
            if lexeme.trim().parse::<f64>().is_err() {
                self.errors.push(
                    Diagnostic::new("PAR072", format!("namba batili: \"{lexeme}\" si muundo sahihi wa desimali"))
                        .with_stage("uchanganuzi")
                        .with_span(line, column),
                );
                return None;
            }
            return Some(Expr::Number(lexeme));
        }

        // [] list literal → Expr::List
        if self.match_tok("[") {
            let line = self.prev().line;
            let mut elements = Vec::new();
            while !self.check("]") && !self.is_eof() {
                elements.push(self.parse_expression()?);
                if !self.match_tok(",") {
                    break;
                }
            }
            self.consume("]", "PAR090", "orodha literal inahitaji ']'")?;
            return Some(Expr::List { elements, line });
        }

        // {} / { k: v, ... } map literal → Expr::Map
        if self.check("{") {
            let first_inside = self.tokens.get(self.pos + 1).map(|u| u.lexeme.as_str());
            let second_inside = self.tokens.get(self.pos + 2).map(|u| u.lexeme.as_str());
            let is_empty_map = first_inside == Some("}");
            let is_map_lit = !is_empty_map && second_inside == Some(":");
            if is_empty_map || is_map_lit {
                let line = self.peek().line;
                self.advance(); // consume {
                let mut entries: Vec<(Expr, Expr)> = Vec::new();
                loop {
                    if self.check("}") || self.is_eof() {
                        break;
                    }
                    let k = self.parse_expression()?;
                    self.consume(":", "PAR092", "kamusi literal inahitaji ':' kati ya ufunguo na thamani")?;
                    let v = self.parse_expression()?;
                    entries.push((k, v));
                    if !self.match_tok(",") {
                        break;
                    }
                }
                self.consume("}", "PAR093", "kamusi literal inahitaji '}'")?;
                return Some(Expr::Map { entries, line });
            }
        }

        if self.check_ident() {
            let t = self.advance();
            let name = t.lexeme.clone();
            let line = t.line;
            let column = t.column;
            // Only parse struct literal when "{ field : expr" appears; "pattern =>" is linganisha arms.
            if self.check("{") {
                let first_inside = self.tokens.get(self.pos + 1).map(|u| u.lexeme.as_str());
                let second_inside = self.tokens.get(self.pos + 2).map(|u| u.lexeme.as_str());
                let is_struct_lit = second_inside == Some(":") || first_inside == Some("}");
                if is_struct_lit && self.match_tok("{") {
                    let (fields, field_positions) = self.parse_struct_literal_fields()?;
                    return Some(Expr::StructLiteral {
                        struct_name: name,
                        fields,
                        field_positions,
                        line,
                    });
                }
            }
            return Some(Expr::Ident { name, line, column });
        }

        self.err_here("PAR071", "usemi usiokubalika");
        None
    }

    fn parse_struct_literal_fields(&mut self) -> Option<(Vec<(String, Expr)>, Vec<(usize, usize)>)> {
        let mut fields = Vec::new();
        let mut field_positions = Vec::new();
        loop {
            if self.match_tok("}") {
                break;
            }
            let fname = self.consume_ident("PAR074", "umbo literal inahitaji jina la uga")?;
            self.consume(":", "PAR075", "umbo literal inahitaji ':' baada ya jina la uga")?;
            let expr = self.parse_expression()?;
            field_positions.push((fname.line, fname.column));
            fields.push((fname.lexeme, expr));
            if !self.match_tok(",") {
                let _ = self.consume("}", "PAR076", "umbo literal inahitaji '}'");
                break;
            }
        }
        Some((fields, field_positions))
    }

    fn skip_top_level(&mut self) {
        while !self.is_eof() {
            if self.check("kazi") || self.check("umma") || self.check("leta") {
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
                        data: Some(TypeExpr { name: "T".to_string() }),
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
                        data: Some(TypeExpr { name: "T".to_string() }),
                        line: 0,
                        column: 0,
                    },
                    EnumVariant {
                        name: "Kosa".to_string(),
                        data: Some(TypeExpr { name: "E".to_string() }),
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
}
