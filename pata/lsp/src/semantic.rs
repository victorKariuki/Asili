use asili_parser::{parse_tokens, Module, Expr, Stmt, ValueType, TypeExpr};
use asili_lexer::tokenize;
use std::collections::HashMap;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokens};
use crate::types::{ScopeContext, TypeInfo, SymbolInfo, SymbolKind, HoverInfo};

/// Convert a TypeExpr name string to ValueType (best effort).
pub fn type_expr_to_value_type(type_expr: &TypeExpr) -> ValueType {
    match type_expr.name.as_str() {
        "Nambari" | "Namba" => ValueType::Namba,
        "Neno" => ValueType::Neno,
        "Ukweli" => ValueType::Ukweli,
        "Herufi" => ValueType::Herufi,
        "Tupu" => ValueType::Tupu,
        "Hamna" => ValueType::Hamna,
        "NambaKuu" => ValueType::NambaKuu,
        "NambaSahihi" => ValueType::NambaSahihi,
        "Wakati" => ValueType::Wakati,
        "Anuani" => ValueType::Anuani,
        name => {
            if name.ends_with('?') {
                ValueType::Chaguo(Box::new(ValueType::Unknown))
            } else {
                ValueType::Struct(name.to_string())
            }
        }
    }
}

pub const TOKEN_TYPES: &[&str] = &[
    "keyword",
    "type",
    "function",
    "variable",
    "parameter",
    "property",
];

pub const TOKEN_MODIFIERS: &[&str] = &["declaration", "readonly"];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum TokenType {
    Keyword = 0,
    Type = 1,
    Function = 2,
    Variable = 3,
    Parameter = 4,
    Property = 5,
}

impl TokenType {
    fn to_u32(self) -> u32 {
        self as u32
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum TokenModifier {
    Declaration = 1,
    Readonly = 2,
}

impl TokenModifier {
    fn to_u32(self) -> u32 {
        self as u32
    }
}

pub struct SemanticAnalyzer {
    module: Module,
    tokens: Vec<SemanticToken>,
    declarations: HashMap<String, (usize, usize, TokenType, u32)>,
    scopes: Vec<ScopeContext>,
    symbol_table: HashMap<String, SymbolInfo>,
    prev_line: usize,
    prev_start: usize,
}

impl SemanticAnalyzer {
    pub fn new(module: Module) -> Self {
        SemanticAnalyzer {
            module,
            tokens: Vec::new(),
            declarations: HashMap::new(),
            scopes: vec![ScopeContext::new(0, None)],
            symbol_table: HashMap::new(),
            prev_line: 0,
            prev_start: 0,
        }
    }

    pub fn analyze(&mut self) -> SemanticTokens {
        self.collect_declarations();
        self.collect_tokens();

        SemanticTokens {
            result_id: None,
            data: self.tokens.clone(),
        }
    }

    fn collect_declarations(&mut self) {
        let functions = self.module.functions.clone();
        let structs = self.module.structs.clone();
        let enums = self.module.enums.clone();
        let traits = self.module.traits.clone();

        for f in &functions {
            self.declarations.insert(
                f.name.clone(),
                (f.line, 1, TokenType::Function, TokenModifier::Declaration.to_u32()),
            );
            self.symbol_table.insert(
                f.name.clone(),
                SymbolInfo {
                    name: f.name.clone(),
                    kind: SymbolKind::Function,
                    type_info: Some(TypeInfo {
                        value_type: type_expr_to_value_type(&f.return_type),
                        scope_depth: 0,
                    }),
                    declaration_line: f.line,
                },
            );
            for p in &f.params {
                self.declarations.insert(
                    p.name.clone(),
                    (p.line, 1, TokenType::Parameter, TokenModifier::Declaration.to_u32()),
                );
            }
        }

        for s in &structs {
            self.declarations.insert(
                s.name.clone(),
                (s.line, 1, TokenType::Type, TokenModifier::Declaration.to_u32()),
            );
            self.symbol_table.insert(
                s.name.clone(),
                SymbolInfo {
                    name: s.name.clone(),
                    kind: SymbolKind::Struct,
                    type_info: None,
                    declaration_line: s.line,
                },
            );
        }

        for e in &enums {
            self.declarations.insert(
                e.name.clone(),
                (e.line, 1, TokenType::Type, TokenModifier::Declaration.to_u32()),
            );
            self.symbol_table.insert(
                e.name.clone(),
                SymbolInfo {
                    name: e.name.clone(),
                    kind: SymbolKind::Struct,
                    type_info: None,
                    declaration_line: e.line,
                },
            );
        }

        for t in &traits {
            self.declarations.insert(
                t.name.clone(),
                (t.line, 1, TokenType::Type, TokenModifier::Declaration.to_u32()),
            );
            self.symbol_table.insert(
                t.name.clone(),
                SymbolInfo {
                    name: t.name.clone(),
                    kind: SymbolKind::Trait,
                    type_info: None,
                    declaration_line: t.line,
                },
            );
        }
    }

    fn collect_tokens(&mut self) {
        let functions = self.module.functions.clone();
        for f in &functions {
            self.push_scope(Some(f.name.clone()));
            for param in &f.params {
                self.current_scope_mut().bind(param.name.clone(), type_expr_to_value_type(&param.ty));
            }
            self.scan_block(&f.body);
            self.pop_scope();
        }
    }

    fn scan_block(&mut self, block: &asili_parser::Block) {
        for stmt in &block.statements {
            self.scan_stmt(stmt);
        }
    }

    fn scan_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, line, ty, .. } => {
                self.add_token(*line, 1, TokenType::Variable, TokenModifier::Declaration.to_u32(), name.len());
                self.scan_expr(value);
                let inferred_type = if let Some(t) = ty {
                    type_expr_to_value_type(t)
                } else {
                    self.infer_expr_type(value)
                };
                self.current_scope_mut().bind(name.clone(), inferred_type);
            }
            Stmt::Assign { name: _, value, .. } => {
                self.scan_expr(value);
            }
            Stmt::If { cond, then_block, else_if, else_block, .. } => {
                self.scan_expr(cond);
                self.push_scope(self.current_scope().parent_fn.clone());
                self.scan_block(then_block);
                self.pop_scope();
                for (c, b) in else_if {
                    self.scan_expr(c);
                    self.push_scope(self.current_scope().parent_fn.clone());
                    self.scan_block(b);
                    self.pop_scope();
                }
                if let Some(b) = else_block {
                    self.push_scope(self.current_scope().parent_fn.clone());
                    self.scan_block(b);
                    self.pop_scope();
                }
            }
            Stmt::While { cond, body, .. } => {
                self.scan_expr(cond);
                self.push_scope(self.current_scope().parent_fn.clone());
                self.scan_block(body);
                self.pop_scope();
            }
            Stmt::For { body, .. } => {
                self.push_scope(self.current_scope().parent_fn.clone());
                self.scan_block(body);
                self.pop_scope();
            }
            Stmt::Match { expr, arms, .. } => {
                self.scan_expr(expr);
                for a in arms {
                    self.push_scope(self.current_scope().parent_fn.clone());
                    self.scan_block(&a.body);
                    self.pop_scope();
                }
            }
            Stmt::Return { value: Some(e), .. } => {
                self.scan_expr(e);
            }
            Stmt::Return { .. } => {}
            Stmt::Expr { expr, .. } => {
                self.scan_expr(expr);
            }
            _ => {}
        }
    }

    fn scan_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name) => {
                if let Some((decl_line, _, token_type, _)) = self.declarations.get(name).copied() {
                    self.add_token(decl_line, 1, token_type, 0, name.len());
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident(_) = &**callee {
                    self.scan_expr(callee);
                }
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Expr::MethodCall { receiver, args, .. } => {
                self.scan_expr(receiver);
                for arg in args {
                    self.scan_expr(arg);
                }
            }
            Expr::Binary { left, right, .. } => {
                self.scan_expr(left);
                self.scan_expr(right);
            }
            Expr::Unary { expr: e, .. } => {
                self.scan_expr(e);
            }
            Expr::Cast { expr: e, .. } => {
                self.scan_expr(e);
            }
            Expr::Index { base, index, .. } => {
                self.scan_expr(base);
                self.scan_expr(index);
            }
            Expr::FieldAccess { receiver, .. } => {
                self.scan_expr(receiver);
            }
            Expr::Group(e) => {
                self.scan_expr(e);
            }
            Expr::Propagate { expr: e, .. } => {
                self.scan_expr(e);
            }
            Expr::List { elements, .. } => {
                for e in elements {
                    self.scan_expr(e);
                }
            }
            Expr::Map { entries, .. } => {
                for (k, v) in entries {
                    self.scan_expr(k);
                    self.scan_expr(v);
                }
            }
            Expr::StructLiteral { fields, .. } => {
                for (_, e) in fields {
                    self.scan_expr(e);
                }
            }
            _ => {}
        }
    }

    fn add_token(&mut self, line: usize, column: usize, token_type: TokenType, modifiers: u32, length: usize) {
        let delta_line = (line as u32).saturating_sub(self.prev_line as u32);
        let delta_start = if line == self.prev_line {
            (column as u32).saturating_sub(self.prev_start as u32)
        } else {
            column as u32
        };
        self.tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length: length as u32,
            token_type: token_type.to_u32(),
            token_modifiers_bitset: modifiers,
        });
        self.prev_line = line;
        self.prev_start = column + length;
    }

    fn push_scope(&mut self, parent_fn: Option<String>) {
        let depth = self.scopes.len();
        self.scopes.push(ScopeContext::new(depth, parent_fn));
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn current_scope_mut(&mut self) -> &mut ScopeContext {
        self.scopes.last_mut().expect("scope stack never empty")
    }

    fn current_scope(&self) -> &ScopeContext {
        self.scopes.last().expect("scope stack never empty")
    }

    /// Infer the type of an expression (basic type inference for literals and identifiers).
    fn infer_expr_type(&self, expr: &Expr) -> ValueType {
        match expr {
            Expr::Number(_) => ValueType::Namba,
            Expr::String(_) => ValueType::Neno,
            Expr::Char(_) => ValueType::Herufi,
            Expr::Bool(_) => ValueType::Ukweli,
            Expr::Hamna => ValueType::Hamna,
            Expr::Ident(name) => {
                self.get_type_at(0, 0, name).unwrap_or(ValueType::Unknown)
            }
            Expr::List { .. } => {
                ValueType::Orodha(Box::new(ValueType::Unknown))
            }
            Expr::Map { .. } => {
                ValueType::Kamusi(Box::new(ValueType::Unknown), Box::new(ValueType::Unknown))
            }
            Expr::Group(e) => self.infer_expr_type(e),
            _ => ValueType::Unknown,
        }
    }

    /// Get type information for a symbol at a given position.
    /// Searches from innermost to outermost scope.
    pub fn get_type_at(&self, _line: usize, _col: usize, name: &str) -> Option<ValueType> {
        for scope in self.scopes.iter().rev() {
            if let Some(type_info) = scope.get(name) {
                return Some(type_info.value_type.clone());
            }
        }
        None
    }

    /// Get symbol information for hover rendering at a token position.
    pub fn get_hover_info(&self, name: &str) -> Option<HoverInfo> {
        if let Some(symbol) = self.symbol_table.get(name) {
            return match symbol.kind {
                SymbolKind::Function => {
                    let func = self.module.functions.iter().find(|f| f.name == name)?;
                    Some(HoverInfo::Function {
                        name: func.name.clone(),
                        params: func.params.iter()
                            .map(|p| (p.name.clone(), type_expr_to_value_type(&p.ty)))
                            .collect(),
                        return_type: type_expr_to_value_type(&func.return_type),
                    })
                }
                SymbolKind::Struct => {
                    let s = self.module.structs.iter().find(|st| st.name == name)?;
                    Some(HoverInfo::Struct {
                        name: s.name.clone(),
                        fields: s.fields.iter()
                            .map(|f| (f.0.clone(), f.1.as_ref().map(type_expr_to_value_type).unwrap_or(ValueType::Unknown)))
                            .collect(),
                    })
                }
                SymbolKind::Trait => {
                    let _t = self.module.traits.iter().find(|tr| tr.name == name)?;
                    Some(HoverInfo::Trait {
                        name: name.to_string(),
                        methods: vec![],
                    })
                }
                _ => None,
            };
        }

        if let Some(type_) = self.get_type_at(0, 0, name) {
            return Some(HoverInfo::Variable {
                name: name.to_string(),
                type_,
            });
        }

        None
    }
}

pub fn analyze_semantic_tokens(source: &str) -> Option<SemanticTokens> {
    let tokens = tokenize(source).ok()?;
    let module = parse_tokens(&tokens).ok()?;
    let mut analyzer = SemanticAnalyzer::new(module);
    Some(analyzer.analyze())
}
