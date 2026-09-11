use asili_parser::{parse_tokens, ForMode, Module, Expr, Pattern, Stmt, ValueType, TypeExpr};
use asili_lexer::tokenize;
use std::collections::{HashMap, HashSet};
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
    "enumMember",
];

pub const TOKEN_MODIFIERS: &[&str] = &["declaration", "readonly", "defaultLibrary"];

// NOTE(syntax-highlighting): "keyword" is intentionally never emitted here. Keyword tokens
// (ikiwa, wakati, weka, ...) are consumed and discarded during parsing — the AST keeps no
// span for them — so recovering their positions would mean threading keyword spans through
// every Stmt variant for no visible gain: the TextMate grammar already scopes every keyword
// correctly on its own (see extensions/vscode/syntaxes/asili.tmLanguage.json). Same reasoning
// for *type references* beyond a struct/enum/trait's own declaration: TypeExpr carries no
// position, but the grammar's `[A-Z][a-zA-Z0-9_]*` rule already colors every occurrence of a
// capitalized identifier as a type, declaration or usage alike, so there's nothing missing.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum TokenType {
    Keyword = 0,
    Type = 1,
    Function = 2,
    Variable = 3,
    Parameter = 4,
    Property = 5,
    EnumMember = 6,
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
    DefaultLibrary = 4,
}

impl TokenModifier {
    fn to_u32(self) -> u32 {
        self as u32
    }
}

/// A token before delta-encoding, keyed by absolute (line, column) so it can be sorted into
/// document order regardless of the (arbitrary) order the analyzer visits declarations/usages
/// in — e.g. `module.functions` doesn't necessarily match physical file order once structs,
/// enums and functions are interleaved in the source.
struct RawToken {
    line: usize,
    column: usize,
    token_type: TokenType,
    modifiers: u32,
    length: usize,
}

pub struct SemanticAnalyzer {
    module: Module,
    raw_tokens: Vec<RawToken>,
    /// Module-level names: functions, structs, enums, traits, constants. These are legitimately
    /// global (Asili has no nested modules), so a flat map is fine here. Stores the modifiers
    /// too (not just the type) so e.g. a module constant's `readonly` modifier carries through
    /// to every usage, not just its declaration site.
    global_decls: HashMap<String, (TokenType, u32)>,
    /// One frame per lexical scope (function body, if/while/for/match-arm block), pushed and
    /// popped in lockstep with `scopes`. Unlike `global_decls`, this is scope-aware so a
    /// parameter or local named `x` in one function can't bleed its token type/modifiers into
    /// an unrelated `x` declared in another function or an outer scope.
    local_decls: Vec<HashMap<String, (TokenType, u32)>>,
    /// Real builtin names from the actual runtime registry (`asili_evaluator::builtins`), used
    /// to give builtin calls a `defaultLibrary` modifier — distinguishing `chapisha(...)` from
    /// a same-shaped user-defined function the way most editors set stdlib calls apart. This is
    /// the authoritative list; unlike the grammar's own hardcoded builtin-function regex or
    /// symbols.rs's completion list (which has drifted to only 12 of the ~50 real builtins),
    /// this can never go stale.
    builtin_names: HashSet<String>,
    scopes: Vec<ScopeContext>,
    symbol_table: HashMap<String, SymbolInfo>,
}

impl SemanticAnalyzer {
    pub fn new(module: Module) -> Self {
        SemanticAnalyzer {
            module,
            raw_tokens: Vec::new(),
            global_decls: HashMap::new(),
            local_decls: vec![HashMap::new()],
            builtin_names: asili_evaluator::builtins::builtins().into_keys().collect(),
            scopes: vec![ScopeContext::new(0, None)],
            symbol_table: HashMap::new(),
        }
    }

    pub fn analyze(&mut self) -> SemanticTokens {
        self.collect_declarations();
        self.collect_tokens();

        self.raw_tokens.sort_by_key(|t| (t.line, t.column));

        let mut data = Vec::with_capacity(self.raw_tokens.len());
        let mut prev_line = 0usize;
        let mut prev_start = 0usize;
        for t in &self.raw_tokens {
            let delta_line = (t.line as u32).saturating_sub(prev_line as u32);
            let delta_start = if t.line == prev_line {
                (t.column as u32).saturating_sub(prev_start as u32)
            } else {
                t.column as u32
            };
            data.push(SemanticToken {
                delta_line,
                delta_start,
                length: t.length as u32,
                token_type: t.token_type.to_u32(),
                token_modifiers_bitset: t.modifiers,
            });
            prev_line = t.line;
            prev_start = t.column;
        }

        SemanticTokens {
            result_id: None,
            data,
        }
    }

    /// `line`/`column` are the 1-based positions the lexer/parser use everywhere else in this
    /// codebase. The LSP semantic-tokens protocol is 0-based, so convert right here — the one
    /// place every token passes through — rather than at each of the ~10 call sites. Getting
    /// this wrong shifts every token down/right by one line/column, which is easy to miss when
    /// there are only a couple of sparse tokens but turns into visible "patches of color in the
    /// wrong place" (e.g. landing on the comment line above the real declaration) once many
    /// tokens are emitted per line.
    fn push_raw(&mut self, line: usize, column: usize, token_type: TokenType, modifiers: u32, length: usize) {
        self.raw_tokens.push(RawToken {
            line: line.saturating_sub(1),
            column: column.saturating_sub(1),
            token_type,
            modifiers,
            length,
        });
    }

    fn collect_declarations(&mut self) {
        let functions = self.module.functions.clone();
        let structs = self.module.structs.clone();
        let enums = self.module.enums.clone();
        let traits = self.module.traits.clone();
        let constants = self.module.constants.clone();

        for f in &functions {
            self.global_decls.insert(f.name.clone(), (TokenType::Function, 0));
            // line 0 marks a synthetic/builtin declaration with no real source position
            // (see standard_enums() below) — never emit a token for one of those.
            if f.line > 0 {
                self.push_raw(f.line, f.column, TokenType::Function, TokenModifier::Declaration.to_u32(), f.name.len());
            }
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
        }

        for s in &structs {
            self.global_decls.insert(s.name.clone(), (TokenType::Type, 0));
            if s.line > 0 {
                self.push_raw(s.line, s.column, TokenType::Type, TokenModifier::Declaration.to_u32(), s.name.len());
            }
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
            self.global_decls.insert(e.name.clone(), (TokenType::Type, 0));
            // Chaguo/Tokeo (standard_enums() in core/parser) are synthesized at line 0 with
            // no real source position — skip those, they'd otherwise paint over real code at
            // the very start of every file once sorted first. Same for their variants below.
            if e.line > 0 {
                self.push_raw(e.line, e.column, TokenType::Type, TokenModifier::Declaration.to_u32(), e.name.len());
            }
            self.symbol_table.insert(
                e.name.clone(),
                SymbolInfo {
                    name: e.name.clone(),
                    kind: SymbolKind::Struct,
                    type_info: None,
                    declaration_line: e.line,
                },
            );
            // Enum variants (Some, Ok, Err, custom ones) are values, not types — a category
            // error the grammar can't avoid since it can only see "any capitalized identifier".
            // Give them their own `enumMember` token here instead.
            for v in &e.variants {
                if v.line > 0 {
                    self.push_raw(v.line, v.column, TokenType::EnumMember, TokenModifier::Declaration.to_u32(), v.name.len());
                }
            }
        }

        for t in &traits {
            self.global_decls.insert(t.name.clone(), (TokenType::Type, 0));
            if t.line > 0 {
                self.push_raw(t.line, t.column, TokenType::Type, TokenModifier::Declaration.to_u32(), t.name.len());
            }
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

        for c in &constants {
            // Module constants are inherently immutable — no `mutable` flag to check, unlike
            // `weka`/`thabiti` locals; `readonly` applies unconditionally, at the declaration
            // and (via global_decls carrying the modifier) at every usage too.
            self.global_decls.insert(c.name.clone(), (TokenType::Variable, TokenModifier::Readonly.to_u32()));
            if c.line > 0 {
                self.push_raw(
                    c.line,
                    c.column,
                    TokenType::Variable,
                    TokenModifier::Declaration.to_u32() | TokenModifier::Readonly.to_u32(),
                    c.name.len(),
                );
            }
            // Bind into the base (module-level) scope so hover can find its type the same way
            // it finds any other variable's, via get_type_at/get_hover_info.
            self.scopes[0].bind(c.name.clone(), type_expr_to_value_type(&c.ty));
        }
    }

    fn collect_tokens(&mut self) {
        let functions = self.module.functions.clone();
        for f in &functions {
            self.push_scope(Some(f.name.clone()));
            for param in &f.params {
                self.current_scope_mut().bind(param.name.clone(), type_expr_to_value_type(&param.ty));
                self.bind_local(param.name.clone(), TokenType::Parameter, 0);
                self.push_raw(param.line, param.column, TokenType::Parameter, TokenModifier::Declaration.to_u32(), param.name.len());
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

    /// Bind a name in the *innermost* active scope, for both token-type lookup (this file)
    /// and type inference (`ScopeContext`, used by hover). Scope-local, so a `weka x` in one
    /// function/block can never bleed its token type or type into an unrelated `x` elsewhere.
    fn bind_local(&mut self, name: String, token_type: TokenType, modifiers: u32) {
        self.local_decls
            .last_mut()
            .expect("local_decls stack never empty")
            .insert(name, (token_type, modifiers));
    }

    /// Resolve a name to its semantic token type + modifiers: innermost-to-outermost through
    /// the active scope stack first (locals/parameters), then module-level declarations
    /// (functions/structs/enums/traits/constants), then real builtins (`defaultLibrary`).
    /// Returns None for anything else (unknown names) — those keep whatever the TextMate
    /// grammar already gives them.
    fn resolve(&self, name: &str) -> Option<(TokenType, u32)> {
        for frame in self.local_decls.iter().rev() {
            if let Some(found) = frame.get(name) {
                return Some(*found);
            }
        }
        if let Some(found) = self.global_decls.get(name) {
            return Some(*found);
        }
        if self.builtin_names.contains(name) {
            return Some((TokenType::Function, TokenModifier::DefaultLibrary.to_u32()));
        }
        None
    }

    fn scan_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let { name, value, line, column, ty, mutable, .. } => {
                self.scan_expr(value);
                let inferred_type = if let Some(t) = ty {
                    type_expr_to_value_type(t)
                } else {
                    self.infer_expr_type(value)
                };
                self.current_scope_mut().bind(name.clone(), inferred_type);
                // `thabiti` (mutable == false) gets the Readonly modifier on its declaration
                // and every later usage — distinguishing it from `weka` bindings is part of
                // what makes richer editors (rust-analyzer, etc.) feel more colorful than a
                // pure-syntax grammar can on its own.
                let modifiers = if *mutable { 0 } else { TokenModifier::Readonly.to_u32() };
                self.bind_local(name.clone(), TokenType::Variable, modifiers);
                self.push_raw(*line, *column, TokenType::Variable, modifiers | TokenModifier::Declaration.to_u32(), name.len());
            }
            Stmt::Assign { name, value, line, column, .. } => {
                if let Some((token_type, modifiers)) = self.resolve(name) {
                    self.push_raw(*line, *column, token_type, modifiers, name.len());
                }
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
            Stmt::For { var, var_column, mode, body, line, .. } => {
                // Element type follows the same rules as core/parser's semantic analyzer:
                // iterating a Orodha<T>/Kamusi<K,V> yields T / Jozi<K,V>; `kutoka ... hadi ...`
                // always yields Namba.
                let var_ty = match mode {
                    ForMode::InExpr(expr) => match self.infer_expr_type(expr) {
                        ValueType::Orodha(inner) => *inner,
                        ValueType::Kamusi(k, v) => ValueType::Jozi(k, v),
                        _ => ValueType::Unknown,
                    },
                    ForMode::Range { .. } => ValueType::Namba,
                };
                self.push_scope(self.current_scope().parent_fn.clone());
                self.current_scope_mut().bind(var.clone(), var_ty);
                self.bind_local(var.clone(), TokenType::Variable, 0);
                self.push_raw(*line, *var_column, TokenType::Variable, TokenModifier::Declaration.to_u32(), var.len());
                if let ForMode::InExpr(expr) = mode {
                    self.scan_expr(expr);
                }
                if let ForMode::Range { start, end } = mode {
                    self.scan_expr(start);
                    self.scan_expr(end);
                }
                self.scan_block(body);
                self.pop_scope();
            }
            Stmt::Match { expr, arms, .. } => {
                self.scan_expr(expr);
                for a in arms {
                    self.push_scope(self.current_scope().parent_fn.clone());
                    self.scan_pattern(&a.pattern);
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
            // NOTE(syntax-highlighting): color this *usage* at its own line/column (now
            // available on Expr::Ident) rather than only a declaration's position — that's
            // what makes every reference to a variable/parameter/function/type light up, not
            // just the one line where it was declared.
            Expr::Ident { name, line, column } => {
                if let Some((token_type, modifiers)) = self.resolve(name) {
                    self.push_raw(*line, *column, token_type, modifiers, name.len());
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Ident { .. } = &**callee {
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
            Expr::FieldAccess { receiver, field, field_line, field_column, .. } => {
                self.scan_expr(receiver);
                // Field names aren't tracked in any declaration map (struct-field validation
                // is core/parser's job, not the LSP's) — this is purely a highlight, emitted
                // unconditionally at the field name's own position.
                self.push_raw(*field_line, *field_column, TokenType::Property, 0, field.len());
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
            Expr::StructLiteral { fields, field_positions, .. } => {
                // field_positions is index-aligned with fields (see the NOTE on
                // Expr::StructLiteral in ast.rs) — zip rather than a shared tuple so the
                // evaluator/analyzer's existing (String, Expr) destructuring never had to change.
                for ((name, e), (line, column)) in fields.iter().zip(field_positions.iter()) {
                    self.push_raw(*line, *column, TokenType::Property, 0, name.len());
                    self.scan_expr(e);
                }
            }
            Expr::EnumConstruct { variant_name, data, line, column, .. } => {
                self.push_raw(*line, *column, TokenType::EnumMember, 0, variant_name.len());
                if let Some(d) = data {
                    self.scan_expr(d);
                }
            }
            _ => {}
        }
    }

    /// Walk a match-arm pattern: bind `Pattern::Ident` names into the arm's own scope (so e.g.
    /// `n` in `Fulani(n) => ...` resolves as a real variable inside the arm body, not just
    /// falling back to the grammar's generic-identifier coloring) and emit `enumMember` tokens
    /// for variant names matched against (`Pattern::Enum`). Called after push_scope and before
    /// scan_block for the arm, so bindings are visible to the body but don't leak past it.
    fn scan_pattern(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Ident { name, line, column } => {
                self.bind_local(name.clone(), TokenType::Variable, 0);
                self.push_raw(*line, *column, TokenType::Variable, TokenModifier::Declaration.to_u32(), name.len());
            }
            Pattern::Enum { variant_name, data, variant_line, variant_column, .. } => {
                self.push_raw(*variant_line, *variant_column, TokenType::EnumMember, 0, variant_name.len());
                if let Some(sub) = data {
                    self.scan_pattern(sub);
                }
            }
            Pattern::Struct { fields, .. } => {
                for (_, sub) in fields {
                    self.scan_pattern(sub);
                }
            }
            Pattern::Jozi(p1, p2) => {
                self.scan_pattern(p1);
                self.scan_pattern(p2);
            }
            Pattern::Wildcard | Pattern::Literal(_) => {}
        }
    }

    fn push_scope(&mut self, parent_fn: Option<String>) {
        let depth = self.scopes.len();
        self.scopes.push(ScopeContext::new(depth, parent_fn));
        self.local_decls.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            self.local_decls.pop();
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
            Expr::Ident { name, .. } => {
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
