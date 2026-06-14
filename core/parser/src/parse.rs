//! Parsing: module, statements, expressions, patterns.

use asili_diagnostics::Diagnostic;
use std::mem;

use crate::{
    AssignOp, Attribute, BinaryOp, Block, Constant, EnumDecl, EnumVariant, Expr, ForMode,
    Function, Import, ImportPath, ImplDecl, MatchArm, Module, Param, Pattern, Stmt, StructDecl,
    TraitDecl, TypeExpr, UnaryOp,
};
use crate::cursor::Parser;

const MAX_RECURSION_DEPTH: usize = 100;

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
        let name = self.consume_ident("PAR040", "thabiti inahitaji jina")?.lexeme;
        let ty = self.parse_type();
        self.consume("=", "PAR041", "thabiti inahitaji '='")?;
        let value = self.parse_expression()?;
        Some(Constant { name, ty, value, line })
    }

    fn parse_import_path(&mut self) -> Option<ImportPath> {
        let module = self.consume_ident("PAR059", "leta inahitaji jina la moduli")?.lexeme;
        if self.match_tok("::") && self.match_tok("{") {
            let mut names = Vec::new();
            loop {
                let t = self.consume_ident("PAR061", "import selective inahitaji jina")?;
                names.push(t.lexeme);
                if self.match_tok("}") {
                    break;
                }
                self.consume(",", "PAR062", "selective import inahitaji ',' kati ya majina")?;
            }
            return Some(ImportPath::Selective {
                module,
                names,
            });
        }
        Some(ImportPath::Full(module))
    }

    fn parse_attribute(&mut self) -> Option<Attribute> {
        self.consume("#", "PAR080", "attribute inahitaji '#'")?;
        self.consume("[", "PAR081", "attribute inahitaji '['")?;
        let name = self.consume_ident("PAR082", "attribute inahitaji jina")?;
        let mut args = None;
        if self.match_tok("(") {
            let mut raw = String::new();
            while !self.is_eof() && !self.check(")") {
                if !raw.is_empty() {
                    raw.push(' ');
                }
                raw.push_str(&self.advance().lexeme);
            }
            self.consume(")", "PAR083", "attribute args inahitaji ')'")?;
            args = Some(raw);
        }
        self.consume("]", "PAR084", "attribute inahitaji ']'")?;
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
                    let _ = self.consume("}", "PAR902", "umbo inahitaji '}'");
                    break;
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
            attrs,
            is_public,
        })
    }

    fn parse_enum_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<EnumDecl> {
        let name = self.consume_ident("PAR905", "jenum inahitaji jina")?;
        let line = name.line;
        let generics = self.parse_generic_names();
        let variants = if self.match_tok("{") {
            let mut vars = Vec::new();
            loop {
                if self.match_tok("}") {
                    break;
                }
                let var_name = self.consume_ident("PAR905", "jenum inahitaji jina la lahaja")?;
                let var_line = var_name.line;
                let data = if self.match_tok("(") {
                    let ty = self.parse_type();
                    self.consume(")", "PAR905", "lahaja inahitaji ')'")?;
                    Some(ty)
                } else {
                    None
                };
                vars.push(EnumVariant {
                    name: var_name.lexeme,
                    data,
                    line: var_line,
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
            is_public,
            attrs,
        })
    }

    fn parse_trait_decl(&mut self, is_public: bool, attrs: Vec<Attribute>) -> Option<TraitDecl> {
        let name = self.consume_ident("PAR903", "sifa inahitaji jina")?;
        let line = name.line;
        self.skip_body();
        Some(TraitDecl {
            name: name.lexeme,
            line,
            attrs,
            is_public,
        })
    }

    fn parse_impl_decl(&mut self, attrs: Vec<Attribute>) -> Option<ImplDecl> {
        let first = self.consume_ident("PAR904", "shughuli ya inahitaji target")?;
        let line = first.line;
        let mut trait_name = None;
        let mut target = first.lexeme.clone();
        if self.match_tok("kwa") {
            trait_name = Some(first.lexeme);
            if let Some(t) = self.consume_ident("PAR905", "shughuli ya kwa inahitaji target") {
                target = t.lexeme;
            }
        }
        let mut body = Vec::new();
        if self.match_tok("{") {
            while self.match_tok("kazi") {
                if let Some(f) = self.parse_function(false, false, vec![]) {
                    if f.name == "kuu" {
                        self.errors.push(
                            Diagnostic::new("PAR077", "kazi kuu haiwezi kuwa ndani ya shughuli")
                                .with_stage("parse")
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
            attrs,
        })
    }

    fn parse_params(&mut self) -> Vec<Param> {
        let mut params = Vec::new();
        if self.check(")") {
            return params;
        }

        loop {
            let Some(name) = self.consume_ident("PAR010", "param inahitaji jina") else {
                break;
            };
            if self.consume(":", "PAR011", "param inahitaji ':'").is_none() {
                break;
            }
            let ty = self.parse_type();
            params.push(Param {
                name: name.lexeme,
                ty,
                line: name.line,
            });

            if self.match_tok(",") {
                continue;
            }
            break;
        }

        params
    }

    fn parse_type(&mut self) -> TypeExpr {
        let mut name = String::new();
        let mut depth = 0usize;
        while !self.is_eof() {
            let l = self.peek().lexeme.as_str();
            if depth == 0 && [",", ")", "{", "}", "=", "->"].contains(&l) {
                break;
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
            let mut d = Diagnostic::new("PAR073", "undani mno").with_stage("parse");
            if !self.is_eof() {
                d = d.with_span(self.peek().line, self.peek().column);
            }
            self.errors.push(d);
            self.depth -= 1;
            return None;
        }
        let result = self.parse_block_inner();
        self.depth -= 1;
        result
    }

    fn parse_block_inner(&mut self) -> Option<Block> {
        self.consume("{", "PAR020", "block inahitaji '{'")?;
        let mut statements = Vec::new();
        while !self.is_eof() && !self.check("}") {
            if let Some(stmt) = self.parse_stmt() {
                statements.push(stmt);
            } else {
                self.pos += 1;
            }
        }
        self.consume("}", "PAR021", "block inahitaji '}'")?;
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

        if self.check_ident() && self.check_n(1, "=") {
            let name = self.advance().lexeme.clone();
            let line = self.prev().line;
            self.advance();
            let expr = self.parse_expression()?;
            return Some(Stmt::Assign {
                name,
                op: AssignOp::Assign,
                value: expr,
                line,
            });
        }

        if self.check_ident() && ["+=", "-=", "*=", "/="].contains(&self.peek_n(1).map(|t| t.lexeme.as_str()).unwrap_or("")) {
            let name = self.advance().lexeme.clone();
            let line = self.prev().line;
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
            });
        }

        let line = self.peek().line;
        let expr = self.parse_expression()?;
        Some(Stmt::Expr { expr, line })
    }

    fn parse_for_stmt(&mut self, label: Option<String>) -> Option<Stmt> {
        let line = self.prev().line;
        let var = self
            .consume_ident("PAR054", "kwa inahitaji variable")
            .map(|t| t.lexeme)?;
        if self.match_tok("katika") {
            let expr = self.parse_expression()?;
            let body = self.parse_block()?;
            return Some(Stmt::For {
                label,
                var,
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
                self.consume(")", "PAR053", "jozi pattern inahitaji ')'")?;
                return Some(Pattern::Jozi(Box::new(p1), Box::new(p2)));
            }
            self.consume(")", "PAR053", "pattern inahitaji ')'")?;
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
                    fields,
                });
            }
            return Some(Pattern::Ident(name));
        }
        self.err_here("PAR053", "pattern si sahihi");
        None
    }

    fn parse_expression(&mut self) -> Option<Expr> {
        self.depth += 1;
        if self.depth > MAX_RECURSION_DEPTH {
            if !self.is_eof() {
                self.errors.push(
                    Diagnostic::new("PAR073", "undani mno")
                        .with_stage("parse")
                        .with_span(self.peek().line, self.peek().column),
                );
            }
            self.depth -= 1;
            return None;
        }
        let result = self.parse_or();
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
        self.parse_binary_left(left, &[("au", BinaryOp::Or)], |p| p.parse_and())
    }

    fn parse_and(&mut self) -> Option<Expr> {
        let left = self.parse_bitwise_or()?;
        self.parse_binary_left(left, &[("na", BinaryOp::And)], |p| p.parse_bitwise_or())
    }

    fn parse_bitwise_or(&mut self) -> Option<Expr> {
        let left = self.parse_bitwise_xor()?;
        self.parse_binary_left(left, &[("au_biti", BinaryOp::BitOr)], |p| p.parse_bitwise_xor())
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
        if self.match_tok("&") {
            let line = self.prev().line;
            let op = if self.match_tok("mut") {
                UnaryOp::BorrowMut
            } else {
                UnaryOp::BorrowImm
            };
            let right = self.parse_unary()?;
            return Some(Expr::Unary {
                op,
                expr: Box::new(right),
                line,
            });
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
                let args = self.parse_paren_args("PAR060", "mwito wa kazi unahitaji ')' ")?;
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
                if let Expr::Ident(enum_name) = expr {
                    let variant_tok = self.consume_ident("PAR080", "jenum variant inahitaji jina")?;
                    let variant_name = variant_tok.lexeme;
                    let line = variant_tok.line;
                    let data = if self.match_tok("(") {
                        let d = self.parse_expression()?;
                        self.consume(")", "PAR081", "jenum variant data inahitaji ')'")?;
                        Some(Box::new(d))
                    } else {
                        None
                    };
                    expr = Expr::EnumConstruct {
                        enum_name,
                        variant_name,
                        data,
                        line,
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
            self.consume(")", "PAR070", "kikundi kinahitaji ')' ")?;
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
                Diagnostic::new("PAR072", "heksadesimali (0x) haiwezekani - tumia namba za desimali tu")
                    .with_stage("parse")
                    .with_span(next_line, next_col),
            );
            return None;
        }
        if (next_lex.starts_with("0b") || next_lex.starts_with("0B")) && next_lex.len() > 2 {
            self.advance();
            self.errors.push(
                Diagnostic::new("PAR072", "binari (0b) haiwezekani - tumia namba za desimali tu")
                    .with_stage("parse")
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
                    Diagnostic::new("PAR072", "namba batili")
                        .with_stage("parse")
                        .with_span(line, column),
                );
                return None;
            }
            return Some(Expr::Number(lexeme));
        }

        if self.check_ident() {
            let t = self.advance();
            let name = t.lexeme.clone();
            let line = t.line;
            // Only parse struct literal when "{ field : expr" appears; "pattern =>" is linganisha arms.
            if self.check("{") {
                let second_inside = self.tokens.get(self.pos + 2).map(|u| u.lexeme.as_str());
                if second_inside == Some(":") && self.match_tok("{") {
                    let fields = self.parse_struct_literal_fields()?;
                    return Some(Expr::StructLiteral {
                        struct_name: name,
                        fields,
                        line,
                    });
                }
            }
            return Some(Expr::Ident(name));
        }

        self.err_here("PAR071", "expression isiyokubalika");
        None
    }

    fn parse_struct_literal_fields(&mut self) -> Option<Vec<(String, Expr)>> {
        let mut fields = Vec::new();
        loop {
            if self.match_tok("}") {
                break;
            }
            let fname = self.consume_ident("PAR074", "umbo literal inahitaji jina la uga")?;
            self.consume(":", "PAR075", "umbo literal inahitaji ':' baada ya jina la uga")?;
            let expr = self.parse_expression()?;
            fields.push((fname.lexeme, expr));
            if !self.match_tok(",") {
                let _ = self.consume("}", "PAR076", "umbo literal inahitaji '}'");
                break;
            }
        }
        Some(fields)
    }

    fn skip_top_level(&mut self) {
        while !self.is_eof() {
            if self.check("kazi") || self.check("umma") || self.check("leta") {
                break;
            }
            self.pos += 1;
        }
    }
}
