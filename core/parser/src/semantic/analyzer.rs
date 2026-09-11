use asili_diagnostics::{ContextMap, Diagnostic, Span};
use std::collections::{HashMap, HashSet};

use crate::{
    AssignOp, Attribute, Block, Expr, FnContract, ForMode, Function, ImportPath, Module, Pattern,
    Stmt, UnaryOp, BinaryOp, ValueType,
};

#[derive(Clone)]
struct Binding {
    ty: ValueType,
    mutable: bool,
    moved: bool,
    imm_borrows: usize,
    mut_borrowed: bool,
    created_at: Span,
    moved_at: Option<Span>,
    borrowed_at: Vec<Span>,
    dropped_at: Option<Span>,
}

struct Analyzer<'a> {
    module: &'a Module,
    require_main: bool,
    errors: Vec<Diagnostic>,
    fn_map: HashMap<String, &'a Function>,
    extern_fn_map: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
    loop_depth: usize,
    /// Per-scope set of variable names that have type Tokeo and have not been consumed (match, ?, or passed to Tokeo param).
    unconsumed_tokeo: Vec<HashSet<String>>,
    /// Per-scope set of variable names that have been dropped.
    dropped_vars: Vec<HashSet<String>>,
    /// `leta` targets accepted in addition to the builtin-module whitelist (SEM007) — module
    /// names a project resolver has already confirmed exist (user modules, path dependencies).
    resolved_modules: HashSet<String>,
    /// This module's own top-level `thabiti` constants (by name). Distinct from
    /// `extern_constants`, which holds constants imported *from other* modules — without this,
    /// a file referencing its own module-level constant failed with SEM045 "jina halijulikani",
    /// even though the same constant worked fine when imported into a different file.
    local_constants: HashMap<String, ValueType>,
}

use super::types::parse_value_type;

#[derive(Copy, Clone)]
enum UseMode {
    Move,
    BorrowImm,
    BorrowMut,
    /// Move out of scope (e.g. return); allows moving even if imm_borrowed.
    Return,
}

impl<'a> Analyzer<'a> {
    fn new(
        module: &'a Module,
        require_main: bool,
        extern_fn_map: HashMap<String, FnContract>,
        extern_constants: HashMap<String, ValueType>,
        resolved_modules: HashSet<String>,
    ) -> Self {
        let mut fn_map = HashMap::new();
        for f in &module.functions {
            fn_map.insert(f.name.clone(), f);
        }
        let mut local_constants = HashMap::new();
        for c in &module.constants {
            local_constants.insert(c.name.clone(), parse_value_type(&c.ty.name));
        }
        Self {
            module,
            require_main,
            errors: Vec::new(),
            fn_map,
            extern_fn_map,
            extern_constants,
            loop_depth: 0,
            unconsumed_tokeo: Vec::new(),
            dropped_vars: Vec::new(),
            resolved_modules,
            local_constants,
        }
    }

    fn get_type_in_scopes(scopes: &[HashMap<String, Binding>], name: &str) -> Option<ValueType> {
        for scope in scopes.iter().rev() {
            if let Some(b) = scope.get(name) {
                return Some(b.ty.clone());
            }
        }
        None
    }

    fn collect_idents_from_expr(expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Ident { name: n, .. } => vec![n.clone()],
            Expr::Group(e) => Self::collect_idents_from_expr(e),
            Expr::Unary { expr: e, .. } => Self::collect_idents_from_expr(e),
            Expr::Binary { left, right, .. } => {
                let mut v = Self::collect_idents_from_expr(left);
                v.extend(Self::collect_idents_from_expr(right));
                v
            }
            Expr::Index { base, index, .. } => {
                let mut v = Self::collect_idents_from_expr(base);
                v.extend(Self::collect_idents_from_expr(index));
                v
            }
            Expr::Call { callee, args, .. } => {
                let mut v = Self::collect_idents_from_expr(callee);
                for a in args {
                    v.extend(Self::collect_idents_from_expr(a));
                }
                v
            }
            Expr::MethodCall { receiver, args, .. } => {
                let mut v = Self::collect_idents_from_expr(receiver);
                for a in args {
                    v.extend(Self::collect_idents_from_expr(a));
                }
                v
            }
            Expr::Propagate { expr: e, .. } => Self::collect_idents_from_expr(e),
            Expr::FieldAccess { receiver, .. } => Self::collect_idents_from_expr(receiver),
            Expr::StructLiteral { fields, .. } => {
                let mut v = Vec::new();
                for (_, e) in fields {
                    v.extend(Self::collect_idents_from_expr(e));
                }
                v
            }
            Expr::Cast { expr: e, .. } => Self::collect_idents_from_expr(e),
            _ => vec![],
        }
    }

    fn mark_tokeo_consumed(&mut self, scopes: &[HashMap<String, Binding>], idents: &[String]) {

        for name in idents {
            if let Some(ty) = Self::get_type_in_scopes(scopes, name) {
                if matches!(ty, ValueType::Tokeo(_, _)) {
                    for set in self.unconsumed_tokeo.iter_mut().rev() {
                        if set.remove(name) {
                            break;
                        }
                    }
                }
            }
        }
    }

    // #[inline(never)]: keeps this out of check_stmt's stack frame — check_stmt recurses once
    // per nesting level of control flow, so a large frame there is disproportionately expensive
    // for deeply-nested source (see phase1_evaluation_depth).
    #[inline(never)]
    fn bind_struct_pattern(
        &self,
        struct_name: &str,
        fields: &[(String, Pattern)],
        scopes: &mut [HashMap<String, Binding>],
        line: usize,
    ) {
        let field_tys: Vec<(String, ValueType)> = self
            .module
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .map(|s| {
                s.fields
                    .iter()
                    .map(|(fname, fty)| {
                        let ty = fty
                            .as_ref()
                            .map(|t| self.type_from_decl(&t.name))
                            .unwrap_or(ValueType::Unknown);
                        (fname.clone(), ty)
                    })
                    .collect()
            })
            .unwrap_or_default();
        for (fname, sub_pat) in fields {
            if let Pattern::Ident { name: bind_name, .. } = sub_pat {
                let ty = field_tys
                    .iter()
                    .find(|(n, _)| n == fname)
                    .map(|(_, t)| t.clone())
                    .unwrap_or(ValueType::Unknown);
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(bind_name.clone(), Binding {
                        ty,
                        mutable: true,
                        moved: false,
                        imm_borrows: 0,
                        mut_borrowed: false,
                        created_at: Span { line, column: 1 },
                        moved_at: None,
                        borrowed_at: Vec::new(),
                        dropped_at: None,
                    });
                }
            }
        }
    }

    #[inline(never)]
    fn bind_jozi_pattern(
        &self,
        first: &Pattern,
        second: &Pattern,
        scopes: &mut [HashMap<String, Binding>],
        line: usize,
    ) {
        for sub_pat in [first, second] {
            if let Pattern::Ident { name: bind_name, .. } = sub_pat {
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(bind_name.clone(), Binding {
                        ty: ValueType::Unknown,
                        mutable: true,
                        moved: false,
                        imm_borrows: 0,
                        mut_borrowed: false,
                        created_at: Span { line, column: 1 },
                        moved_at: None,
                        borrowed_at: Vec::new(),
                        dropped_at: None,
                    });
                }
            }
        }
    }

    fn run(&mut self) {
        let mut has_main = false;
        let allowed_attrs = ["jaribio", "sharti", "ndani", "kiunganishi"];
        for imp in &self.module.imports {
            let mod_name = match &imp.path {
                ImportPath::Full(s) => s.as_str(),
                ImportPath::Selective { module, .. } => module.as_str(),
            };
            let allowed = matches!(
                mod_name,
                "msingi" | "mfumo" | "majira" | "matumizi" | "faili" | "hisabati"
                    | "runtime" | "syscall" | "kiungo" | "sambamba" | "kasha_gc"
            ) || self.resolved_modules.contains(mod_name);
            if !allowed {
                self.errors.push(
                    Diagnostic::new("SEM007", format!("moduli haijulikani: {}", mod_name))
                        .with_stage("semantiki")
                        .with_span(imp.line, 1),
                );
            }
        }

        for f in &self.module.functions {
            self.check_attrs(&f.attrs, &allowed_attrs);
            if f.name == "kuu" {
                has_main = true;
                self.check_main_sig(f);
            }
            self.check_function(f);
        }
        for e in &self.module.enums {
            self.check_attrs(&e.attrs, &allowed_attrs);
            let mut seen = std::collections::HashSet::new();
            for variant in &e.variants {
                if !seen.insert(&variant.name) {
                    self.errors.push(
                        Diagnostic::new("SEM093", format!("jenum '{}' ina kigezo mara mbili: {}", e.name, variant.name))
                            .with_stage("semantiki")
                            .with_span(e.line, 1),
                    );
                }
            }
        }
        for s in &self.module.structs {
            self.check_attrs(&s.attrs, &allowed_attrs);
            let mut seen = std::collections::HashSet::new();
            for (fname, _) in &s.fields {
                if !seen.insert(fname) {
                    self.errors.push(
                        Diagnostic::new("SEM092", format!("umbo '{}' ina uga mara mbili: {}", s.name, fname))
                            .with_stage("semantiki")
                            .with_span(s.line, 1),
                    );
                }
            }
        }
        for t in &self.module.traits {
            self.check_attrs(&t.attrs, &allowed_attrs);
        }
        for i in &self.module.impls {
            self.check_attrs(&i.attrs, &allowed_attrs);
            for f in &i.body {
                if let Some(first) = f.params.first() {
                    if first.name != "self" {
                        self.errors.push(
                            Diagnostic::new("SEM100", "njia ya shughuli inahitaji hoja ya kwanza 'self'")
                                .with_stage("semantiki")
                                .with_span(f.line, 1),
                        );
                    }
                } else {
                    self.errors.push(
                        Diagnostic::new("SEM101", "njia ya shughuli inahitaji angalau hoja 'self'")
                            .with_stage("semantiki")
                            .with_span(f.line, 1),
                    );
                }
            }
        }

        if self.require_main && !has_main {
            self.errors
                .push(Diagnostic::new("SEM000", "hakuna kazi kuu iliyoonekana").with_stage("semantiki"));
        }
    }

    fn check_attrs(&mut self, attrs: &[Attribute], allowed: &[&str]) {
        for a in attrs {
            if !allowed.contains(&a.name.as_str()) {
                self.errors.push(
                    Diagnostic::new("SEM008", format!("kiambatanisho haijulikani: {}", a.name))
                        .with_stage("semantiki")
                        .with_span(a.line, 1),
                );
            }
        }
    }

    fn check_main_sig(&mut self, f: &Function) {
        if f.return_type.name.trim() != "Tupu" {
            self.errors.push(
                Diagnostic::new("SEM001", "kazi kuu lazima irudishe Tupu")
                    .with_stage("semantiki")
                    .with_span(f.line, 1),
            );
        }
        if f.params.len() != 1 {
            self.errors.push(
                Diagnostic::new("SEM002", "sahihi ya kazi kuu ni kazi kuu(hoja: Orodha<Neno>) -> Tupu")
                    .with_stage("semantiki")
                    .with_span(f.line, 1),
            );
            return;
        }
        let p = &f.params[0];
        if p.name != "hoja" || p.ty.name.replace(' ', "") != "Orodha<Neno>" {
            self.errors.push(
                Diagnostic::new("SEM002", "sahihi ya kazi kuu ni kazi kuu(hoja: Orodha<Neno>) -> Tupu")
                    .with_stage("semantiki")
                    .with_span(f.line, 1),
            );
        }
    }

    fn check_function(&mut self, f: &Function) {
        if f.return_type.name.contains("&") || f.return_type.name.contains("Rejeo") {
            self.errors.push(
                Diagnostic::new(
                    "SEM120",
                    "lifetime inference kamili bado haijatekelezwa kwa return references",
                )
                .with_stage("semantiki")
                .with_span(f.line, 1),
            );
        }

        let mut scopes: Vec<HashMap<String, Binding>> = vec![HashMap::new()];
        self.unconsumed_tokeo = vec![HashSet::new()];
        self.dropped_vars = vec![HashSet::new()];
        for p in &f.params {
            let ty = self.type_from_decl(&p.ty.name);
            scopes[0].insert(
                p.name.clone(),
                Binding {
                    ty,
                    mutable: false,
                    moved: false,
                    imm_borrows: 0,
                    mut_borrowed: false,
                    created_at: Span {
                        line: p.line,
                        column: 1,
                    },
                    moved_at: None,
                    borrowed_at: Vec::new(),
                    dropped_at: None,
                },
            );
        }

        self.check_block(&f.body, &mut scopes, &f.return_type.name, true);

        if let Some(set) = self.unconsumed_tokeo.first() {
            for name in set {
                if let Some(b) = scopes[0].get(name) {
                    self.errors.push(
                        Diagnostic::new("SEM048", "Tokeo haukutumiwa — lazima ulinganishe au utumie ?")
                            .with_stage("semantiki")
                            .with_span(b.created_at.line, b.created_at.column),
                    );
                }
            }
        }
    }

    fn check_block(
        &mut self,
        block: &Block,
        scopes: &mut Vec<HashMap<String, Binding>>,
        return_type: &str,
        root: bool,
    ) {
        if !root {
            scopes.push(HashMap::new());
            self.unconsumed_tokeo.push(HashSet::new());
            self.dropped_vars.push(HashSet::new());
        }
        for stmt in &block.statements {
            self.check_stmt(stmt, scopes, return_type);
        }
        if !root {
            if let Some(set) = self.unconsumed_tokeo.last() {
                for name in set {
                    if let Some(scope) = scopes.last() {
                        if let Some(b) = scope.get(name) {
                            self.errors.push(
                                Diagnostic::new("SEM048", "Tokeo haukutumiwa — lazima ulinganishe au utumie ?")
                                    .with_stage("semantiki")
                                    .with_span(b.created_at.line, b.created_at.column),
                            );
                        }
                    }
                }
            }
            scopes.pop();
            self.unconsumed_tokeo.pop();
            self.dropped_vars.pop();
        }
    }

    fn check_stmt(
        &mut self,
        stmt: &Stmt,
        scopes: &mut Vec<HashMap<String, Binding>>,
        return_type: &str,
    ) {
        match stmt {
            Stmt::Let {
                mutable,
                name,
                ty,
                value,
                line,
                ..
            } => {
                let inferred = self.check_expr(value, scopes, UseMode::Return);
                let declared = ty
                    .as_ref()
                    .map(|t| self.type_from_decl(&t.name))
                    .unwrap_or(inferred.clone());
                if ty.is_some() && !self.compatible(&declared, &inferred) {
                    self.errors.push(
                        Diagnostic::new("SEM010", "aina ya weka/thabiti haitalingana na thamani")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(
                        name.clone(),
                        Binding {
                            ty: declared.clone(),
                            mutable: *mutable,
                            moved: false,
                            imm_borrows: 0,
                            mut_borrowed: false,
                            created_at: Span {
                                line: *line,
                                column: 1,
                            },
                            moved_at: None,
                            borrowed_at: Vec::new(),
                            dropped_at: None,
                        },
                    );
                }
                if matches!(declared, ValueType::Tokeo(_, _)) {
                    if let Some(set) = self.unconsumed_tokeo.last_mut() {
                        set.insert(name.clone());
                    }
                }
            }
            Stmt::Assign {
                name,
                op,
                value,
                line,
                ..
            } => {
                let rhs_ty = self.check_expr(value, scopes, UseMode::Return);
                let mut found = false;
                for scope in scopes.iter_mut().rev() {
                    if let Some(b) = scope.get_mut(name) {
                        found = true;
                        if !b.mutable {
                            self.errors.push(
                                Diagnostic::new("SEM011", "haiwezekani kubadilisha thabiti")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        if b.mut_borrowed {
                            self.errors.push(
                                Diagnostic::new("SEM012", "haiwezekani kuweka thamani wakati jina limeazimwa")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        if b.imm_borrows > 0 {
                            b.imm_borrows = 0;
                            b.borrowed_at.clear();
                        }
                        match op {
                            AssignOp::Assign => {
                                self.check_type_compatibility(
                                    &b.ty,
                                    &rhs_ty,
                                    "SEM013",
                                    "aina ya thamani mpya haitalingani na aina ya jina".to_string(),
                                    Span { line: *line, column: 1 },
                                );
                                b.moved = false;
                                b.moved_at = None;
                            }
                            AssignOp::AddAssign => {
                                let ok = (b.ty == ValueType::Namba && rhs_ty == ValueType::Namba)
                                    || (b.ty == ValueType::Neno && rhs_ty == ValueType::Neno);
                                if !ok {
                                    self.errors.push(
                                        Diagnostic::new(
                                            "SEM014",
                                            "+= inahitaji (Namba, Namba) au (Neno, Neno) pekee",
                                        )
                                        .with_stage("semantiki")
                                        .with_span(*line, 1),
                                    );
                                }
                            }
                            _ => {
                                if b.ty != ValueType::Namba || rhs_ty != ValueType::Namba {
                                    let op_sym = match op {
                                        AssignOp::SubAssign => "-=",
                                        AssignOp::MulAssign => "*=",
                                        AssignOp::DivAssign => "/=",
                                        _ => "muundo wa kuchanganya",
                                    };
                                    self.errors.push(
                                        Diagnostic::new("SEM014", format!("{op_sym} inahitaji Namba pande zote mbili"))
                                            .with_stage("semantiki")
                                            .with_span(*line, 1),
                                    );
                                }
                            }
                        }
                        break;
                    }
                }
                if !found {
                    self.errors.push(
                        Diagnostic::new("SEM015", format!("jina halijulikani: {name}"))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_if,
                else_block,
                line,
            } => {
                let ty = self.check_expr(cond, scopes, UseMode::Move);
                if ty != ValueType::Ukweli {
                    self.errors.push(
                        Diagnostic::new("SEM020", "sharti la ikiwa lazima liwe Ukweli")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
                self.check_block(then_block, scopes, return_type, false);
                for (c, b) in else_if {
                    let ty = self.check_expr(c, scopes, UseMode::Move);
                    if ty != ValueType::Ukweli {
                        self.errors.push(
                            Diagnostic::new("SEM021", "sharti la au_ikiwa lazima liwe Ukweli")
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                    }
                    self.check_block(b, scopes, return_type, false);
                }
                if let Some(b) = else_block {
                    self.check_block(b, scopes, return_type, false);
                }
            }
            Stmt::While { label: _, cond, body, line } => {
                let ty = self.check_expr(cond, scopes, UseMode::Move);
                if ty != ValueType::Ukweli {
                    self.errors.push(
                        Diagnostic::new("SEM022", "sharti la wakati lazima liwe Ukweli")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
                self.loop_depth += 1;
                self.check_block(body, scopes, return_type, false);
                self.loop_depth -= 1;
            }
            Stmt::For {
                var,
                mode,
                body,
                line,
                ..
            } => {
                let var_ty = match mode {
                    ForMode::InExpr(expr) => {
                        let iter_ty = self.check_expr(expr, scopes, UseMode::BorrowImm);
                        match iter_ty {
                            ValueType::Orodha(inner) => *inner,
                            ValueType::Kamusi(k, v) => ValueType::Jozi(k, v),
                            _ => ValueType::Namba,
                        }
                    }
                    ForMode::Range { start, end } => {
                        let st = self.check_expr(start, scopes, UseMode::Move);
                        let en = self.check_expr(end, scopes, UseMode::Move);
                        if st != ValueType::Namba || en != ValueType::Namba {
                            self.errors.push(
                                Diagnostic::new("SEM048", "kwa kutoka/hadi inahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Namba
                    }
                };
                self.loop_depth += 1;
                scopes.push(HashMap::new());
                self.unconsumed_tokeo.push(HashSet::new());
                self.dropped_vars.push(HashSet::new());
                if let Some(scope) = scopes.last_mut() {
                    scope.insert(
                        var.clone(),
                        Binding {
                            ty: var_ty,
                            mutable: true,
                            moved: false,
                            imm_borrows: 0,
                            mut_borrowed: false,
                            created_at: Span { line: *line, column: 1 },
                            moved_at: None,
                            borrowed_at: Vec::new(),
                            dropped_at: None,
                        },
                    );
                }
                self.check_block(body, scopes, return_type, true);
                if let Some(set) = self.unconsumed_tokeo.last() {
                    for name in set {
                        if let Some(scope) = scopes.last() {
                            if let Some(b) = scope.get(name) {
                                self.errors.push(
                                    Diagnostic::new("SEM048", "Tokeo haukutumiwa — lazima ulinganishe au utumie ?")
                                        .with_stage("semantiki")
                                        .with_span(b.created_at.line, b.created_at.column),
                                );
                            }
                        }
                    }
                }
                scopes.pop();
                self.unconsumed_tokeo.pop();
                self.dropped_vars.pop();
                self.loop_depth -= 1;
            }
            Stmt::Match { expr, arms, line } => {
                let _ = self.check_expr(expr, scopes, UseMode::Move);
                let idents = Self::collect_idents_from_expr(expr);
                self.mark_tokeo_consumed(scopes, &idents);
                for a in arms {
                    scopes.push(HashMap::new());
                    match &a.pattern {
                        Pattern::Literal(e) => {
                            let _ = self.check_expr(e, scopes, UseMode::Move);
                        }
                        Pattern::Enum { enum_name, variant_name, data, .. } => {
                            // Validate that the enum exists
                            if !self.module.enums.iter().any(|e| &e.name == enum_name) {
                                self.errors.push(
                                    Diagnostic::new("SEM094", format!("jenum '{enum_name}' haijulikani"))
                                        .with_stage("semantiki")
                                        .with_span(*line, 1),
                                );
                            }
                            // Validate that the variant exists and bind data pattern variables
                            if let Some(e) = self.module.enums.iter().find(|en| &en.name == enum_name) {
                                if !e.variants.iter().any(|v| &v.name == variant_name) {
                                    self.errors.push(
                                        Diagnostic::new("SEM095", format!("kigezo '{variant_name}' haipo katika jenum '{enum_name}'"))
                                            .with_stage("semantiki")
                                            .with_span(*line, 1),
                                    );
                                }
                                // Bind data pattern variable with the variant's inner type
                                if let Some(data_pat) = data {
                                    let inner_ty = e.variants.iter()
                                        .find(|v| &v.name == variant_name)
                                        .and_then(|v| v.data.as_ref())
                                        .map(|ty| self.type_from_decl(&ty.name))
                                        .unwrap_or(ValueType::Unknown);
                                    if let Pattern::Ident { name: bind_name, .. } = data_pat.as_ref() {
                                        if let Some(scope) = scopes.last_mut() {
                                            scope.insert(bind_name.clone(), Binding {
                                                ty: inner_ty,
                                                mutable: true,
                                                moved: false,
                                                imm_borrows: 0,
                                                mut_borrowed: false,
                                                created_at: Span { line: *line, column: 1 },
                                                moved_at: None,
                                                borrowed_at: Vec::new(),
                                                dropped_at: None,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        Pattern::Struct { struct_name, fields, .. } => {
                            self.bind_struct_pattern(struct_name, fields, scopes, *line);
                        }
                        Pattern::Jozi(first, second) => {
                            self.bind_jozi_pattern(first, second, scopes, *line);
                        }
                        Pattern::Wildcard | Pattern::Ident { .. } => {}
                    }
                    self.check_block(&a.body, scopes, return_type, false);
                    scopes.pop();
                }
                if arms.is_empty() {
                    self.errors.push(
                        Diagnostic::new("SEM023", "linganisha inahitaji angalau mkono mmoja")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                } else {
                    let has_wildcard = arms.iter().any(|a| matches!(a.pattern, Pattern::Wildcard));
                    let has_ident_catch = arms.iter().any(|a| matches!(a.pattern, Pattern::Ident { .. }));
                    if !has_wildcard && !has_ident_catch {
                        // Check if all variants of the matched enum are covered
                        let enum_name_from_arms = arms.iter().find_map(|a| {
                            if let Pattern::Enum { enum_name, .. } = &a.pattern {
                                Some(enum_name.as_str())
                            } else {
                                None
                            }
                        });
                        let all_covered = if let Some(ename) = enum_name_from_arms {
                            if let Some(e) = self.module.enums.iter().find(|e| e.name == ename) {
                                e.variants.iter().all(|v| {
                                    arms.iter().any(|a| {
                                        matches!(&a.pattern, Pattern::Enum { variant_name, .. } if variant_name == &v.name)
                                    })
                                })
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                        if !all_covered {
                            self.errors.push(
                                Diagnostic::new("SEM023", "linganisha haishughulikii kesi zote zinazowezekana — ongeza _ => {} kama chaguo-msingi")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                    }
                }
            }
            Stmt::Break { label: _, line } => {
                if self.loop_depth == 0 {
                    self.errors.push(
                        Diagnostic::new("SEM024", "vunja inatumika nje ya kitanzi")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
            }
            Stmt::Continue { line, .. } => {
                if self.loop_depth == 0 {
                    self.errors.push(
                        Diagnostic::new("SEM025", "endelea inatumika nje ya kitanzi")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                }
            }
            Stmt::Return { value, line } => {
                let got = value
                    .as_ref()
                    .map(|e| self.check_expr(e, scopes, UseMode::Return))
                    .unwrap_or(ValueType::Tupu);
                let want = self.type_from_decl(return_type);
                self.check_type_compatibility(
                    &want,
                    &got,
                    "SEM026",
                    "Sahihi ya kazi haitalingana na aina ya rejesha".to_string(),
                    Span { line: *line, column: 1 },
                );
            }
            Stmt::Drop { name, line } => {
                let mut found = false;
                let mut already_dropped = false;
                for scope in scopes.iter_mut().rev() {
                    if let Some(b) = scope.get_mut(name) {
                        found = true;
                        if b.dropped_at.is_some() {
                            already_dropped = true;
                        } else {
                            b.moved = true;
                            b.moved_at = Some(Span {
                                line: *line,
                                column: 1,
                            });
                            b.dropped_at = Some(Span {
                                line: *line,
                                column: 1,
                            });
                        }
                        break;
                    }
                }
                if !found {
                    self.errors.push(
                        Diagnostic::new("SEM027", format!("tupa inatumia jina lisilojulikana: {name}"))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                } else if already_dropped {
                    self.errors.push(
                        Diagnostic::new("SEM029", format!("'{name}' tayari imetupwa — haiwezekani kutupa tena"))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                } else {
                    // Track this variable as dropped in the current scope
                    if let Some(set) = self.dropped_vars.last_mut() {
                        set.insert(name.clone());
                    }
                }
            }
            Stmt::Expr { expr, .. } => {
                let _ = self.check_expr(expr, scopes, UseMode::Move);
            }
        }
    }

    fn check_expr(
        &mut self,
        expr: &Expr,
        scopes: &mut Vec<HashMap<String, Binding>>,
        mode: UseMode,
    ) -> ValueType {
        match expr {
            Expr::Number(_) => ValueType::Namba,
            Expr::String(_) => ValueType::Neno,
            Expr::Bool(_) => ValueType::Ukweli,
            Expr::Char(_) => ValueType::Herufi,
            Expr::Hamna => ValueType::Hamna,
            Expr::Group(e) => self.check_expr(e, scopes, mode),
            Expr::Ident { name, .. } => self.use_ident(name, scopes, mode),
            Expr::Unary { op, expr, line } => {
                let t = match op {
                    UnaryOp::BorrowImm => self.check_expr(expr, scopes, UseMode::BorrowImm),
                    UnaryOp::BorrowMut => self.check_expr(expr, scopes, UseMode::BorrowMut),
                    UnaryOp::Jaribu => self.check_expr(expr, scopes, UseMode::Move),
                    _ => self.check_expr(expr, scopes, UseMode::Move),
                };
                match op {
                    UnaryOp::Neg => {
                        if t != ValueType::Namba {
                            self.errors.push(
                                Diagnostic::new("SEM030", "- inahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Namba
                    }
                    UnaryOp::Not => {
                        if t != ValueType::Ukweli {
                            self.errors.push(
                                Diagnostic::new("SEM031", "siyo inahitaji Ukweli")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Ukweli
                    }
                    UnaryOp::BitNot => {
                        if t != ValueType::Namba {
                            self.errors.push(
                                Diagnostic::new("SEM036", "siyo_biti inahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Namba
                    }
                    UnaryOp::BorrowImm => ValueType::Rejeo(Box::new(t), false),
                    UnaryOp::BorrowMut => ValueType::Rejeo(Box::new(t), true),
                    UnaryOp::Jaribu => match t {
                        ValueType::Tokeo(ok, _) => *ok,
                        ValueType::Chaguo(inner) => *inner,
                        _ => {
                            self.errors.push(
                                Diagnostic::new("SEM032", "jaribu inahitaji Tokeo/Chaguo")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                            ValueType::Unknown
                        }
                    },
                }
            }
            Expr::Binary { left, op, right, line } => {
                let l = self.check_expr(left, scopes, UseMode::BorrowImm);
                let r = self.check_expr(right, scopes, UseMode::BorrowImm);
                match op {
                    BinaryOp::Add => {
                        // TypeVar/Unknown are unresolved-generic placeholders (e.g. a pattern
                        // binding for Tokeo<T,E>'s `E`) — treat them as a wildcard here too,
                        // matching `compatible()`'s existing rule, so a genuinely-Neno value
                        // whose static type couldn't be resolved isn't rejected by +.
                        let is_wild = |t: &ValueType| matches!(t, ValueType::TypeVar(_) | ValueType::Unknown);
                        if (l == ValueType::Neno || is_wild(&l)) && (r == ValueType::Neno || is_wild(&r)) && !(is_wild(&l) && is_wild(&r)) {
                            ValueType::Neno
                        } else if (l == ValueType::Namba || is_wild(&l)) && (r == ValueType::Namba || is_wild(&r)) && !(is_wild(&l) && is_wild(&r)) {
                            ValueType::Namba
                        } else {
                            self.errors.push(
                                Diagnostic::new(
                                    "SEM033",
                                    "opereta '+' inahitaji (Namba, Namba) au (Neno, Neno) pekee",
                                )
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                            );
                            ValueType::Unknown
                        }
                    }
                    BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::Pow => {
                        if l != ValueType::Namba || r != ValueType::Namba {
                            self.errors.push(
                                Diagnostic::new("SEM033", "opereta wa hisabati unahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Namba
                    }
                    BinaryOp::Eq | BinaryOp::Ne => {
                        if !self.compatible(&l, &r) {
                            self.errors.push(
                                Diagnostic::new("SEM034", "ulinganisho unahitaji aina zinazolingana")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Ukweli
                    }
                    BinaryOp::Gt | BinaryOp::Lt | BinaryOp::Ge | BinaryOp::Le => {
                        let ok = (l == ValueType::Namba && r == ValueType::Namba)
                            || (l == ValueType::Neno && r == ValueType::Neno);
                        if !ok {
                            self.errors.push(
                                Diagnostic::new(
                                    "SEM034",
                                    "opereta wa kulinganisha (<, >, <=, >=) inahitaji (Namba, Namba) au (Neno, Neno) pekee",
                                )
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                            );
                        }
                        ValueType::Ukweli
                    }
                    BinaryOp::And | BinaryOp::Or => {
                        if l != ValueType::Ukweli || r != ValueType::Ukweli {
                            self.errors.push(
                                Diagnostic::new("SEM035", "na/au inahitaji Ukweli")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Ukweli
                    }
                    BinaryOp::BitAnd | BinaryOp::BitOr | BinaryOp::BitXor | BinaryOp::Shl | BinaryOp::Shr => {
                        if l != ValueType::Namba || r != ValueType::Namba {
                            self.errors.push(
                                Diagnostic::new("SEM037", "opereta wa biti unahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Namba
                    }
                }
            }
            Expr::Cast { expr, ty, .. } => {
                let _ = self.check_expr(expr, scopes, UseMode::Move);
                let target = self.type_from_decl(&ty.name);
                let s = ty.name.replace(' ', "");
                let fallible = s.starts_with("Biti") || s.starts_with("uBiti");
                if fallible {
                    ValueType::Chaguo(Box::new(target))
                } else {
                    target
                }
            }
            Expr::Call { callee, args, line } => {
                let callee_name = if let Expr::Ident { name: n, .. } = &**callee {
                    Some(n.clone())
                } else {
                    None
                };
                let callee_ty = if callee_name.is_some() {
                    ValueType::Unknown
                } else {
                    self.check_expr(callee, scopes, UseMode::Move)
                };
                if let Some(name) = callee_name {
                    if let Some(f) = self.fn_map.get(&name).cloned() {
                        let use_mode = UseMode::BorrowImm;
                        for a in args {
                            let _ = self.check_expr(a, scopes, use_mode);
                        }
                        if f.params.len() != args.len() {
                            self.errors.push(
                                Diagnostic::new("SEM036", format!("mwito wa {name} una idadi tofauti ya hoja"))
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        } else {
                            for (arg, param) in args.iter().zip(f.params.iter()) {
                                if let Expr::Ident { name: n, .. } = arg {
                                    let pt = self.type_from_decl(&param.ty.name);
                                    if matches!(pt, ValueType::Tokeo(_, _)) {
                                        self.mark_tokeo_consumed(scopes, std::slice::from_ref(n));
                                    }
                                }
                            }
                        }
                        return self.type_from_decl(&f.return_type.name);
                    }
                    if let Some(sig) = self.extern_fn_map.get(&name).cloned() {
                        let variadic = name == "orodha";
                        // Evaluate all arg types upfront for both validation and generic instantiation.
                        let arg_types: Vec<ValueType> = args
                            .iter()
                            .map(|a| self.check_expr(a, scopes, UseMode::Move))
                            .collect();
                        if !variadic && sig.params.len() != arg_types.len() {
                            self.errors.push(
                                Diagnostic::new(
                                    "SEM046",
                                    format!("mwito wa {name} una idadi tofauti ya hoja (stdlib)"),
                                )
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                            );
                        } else if !variadic {
                            for (idx, (got, want)) in
                                arg_types.iter().zip(sig.params.iter()).enumerate()
                            {
                                if !self.compatible(want, got) {
                                    self.errors.push(
                                        Diagnostic::new(
                                            "SEM047",
                                            format!("aina ya hoja #{idx} kwenye {name} haitalingana"),
                                        )
                                        .with_stage("semantiki")
                                        .with_span(*line, 1),
                                    );
                                }
                                if let Expr::Ident { name: n, .. } = &args[idx] {
                                    if matches!(want, ValueType::Tokeo(_, _)) {
                                        self.mark_tokeo_consumed(scopes, std::slice::from_ref(n));
                                    }
                                }
                            }
                        }
                        // Instantiate generic return types from actual argument types so that
                        // orodha(1,2,3) → Orodha(Namba) and jozi("a", kweli) → Jozi(Neno, Ukweli)
                        // instead of Orodha(Unknown)/Jozi(Unknown,Unknown) which cause SEM-INF.
                        return match name.as_str() {
                            "orodha" => {
                                let elem = arg_types
                                    .into_iter()
                                    .find(|t| !matches!(t, ValueType::Unknown))
                                    .unwrap_or(ValueType::Unknown);
                                ValueType::Orodha(Box::new(elem))
                            }
                            "jozi" if arg_types.len() == 2 => ValueType::Jozi(
                                Box::new(arg_types[0].clone()),
                                Box::new(arg_types[1].clone()),
                            ),
                            "tokeo" if !arg_types.is_empty() => ValueType::Tokeo(
                                Box::new(arg_types[0].clone()),
                                Box::new(ValueType::Unknown),
                            ),
                            "chaguo" if !arg_types.is_empty() => {
                                ValueType::Chaguo(Box::new(arg_types[0].clone()))
                            }
                            _ => sig.ret.clone(),
                        };
                    }
                    self.errors.push(
                        Diagnostic::new("SEM037", format!("kazi haijulikani: {name}"))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                    return ValueType::Unknown;
                }
                if matches!(callee_ty, ValueType::Unknown) {
                    ValueType::Unknown
                } else {
                    self.errors.push(
                        Diagnostic::new("SEM038", "mwito wa kazi unahitaji jina")
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                    ValueType::Unknown
                }
            }
            // Method call type-checking.
            Expr::MethodCall {
                receiver,
                method_name,
                args,
                line,
            } => {
                let receiver_ty = self.check_expr(receiver, scopes, UseMode::BorrowImm);
                let receiver_ty_name = match &receiver_ty {
                    ValueType::Struct(name) => name.clone(),
                    ValueType::Neno => "Neno".to_string(),
                    ValueType::Orodha(_) => "Orodha".to_string(),
                    ValueType::Kamusi(_, _) => "Kamusi".to_string(),
                    ValueType::Jozi(_, _) => "Jozi".to_string(),
                    ValueType::Chaguo(_) => "Chaguo".to_string(),
                    ValueType::Tokeo(_, _) => "Tokeo".to_string(),
                    ValueType::KashaGC(_) => "Kasha_GC".to_string(),
                    _ => {
                        self.errors.push(
                            Diagnostic::new(
                                "SEM039",
                                format!("aina '{}' haina njia", receiver_ty),
                            )
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                        );
                        return ValueType::Unknown;
                    }
                };
                let is_enum = self.module.enums.iter().any(|e| e.name == receiver_ty_name);
                let _is_struct = self.module.structs.iter().any(|s| s.name == receiver_ty_name);
                let is_builtin = matches!(receiver_ty, ValueType::Neno | ValueType::Orodha(_) | ValueType::Kamusi(_, _) | ValueType::Jozi(_, _) | ValueType::Chaguo(_) | ValueType::Tokeo(_, _) | ValueType::KashaGC(_));
                if is_builtin {
                    for arg in args {
                        let _ = self.check_expr(arg, scopes, UseMode::Move);
                    }
                    return match (receiver_ty, method_name.as_str()) {
                        (ValueType::Neno, "clona") => ValueType::Neno,
                        (ValueType::Neno, "urefu" | "biti_ngapi") => ValueType::Namba,
                        (ValueType::Neno, "kwa_herufi_ndogo" | "kwa_herufi_kubwa" | "badilisha") => ValueType::Neno,
                        (ValueType::Neno, "anza_na" | "maliza_na") => ValueType::Ukweli,
                        (ValueType::Neno, "gawanya") => ValueType::Orodha(Box::new(ValueType::Neno)),
                        (ValueType::Neno, "kata") => ValueType::Neno,
                        (ValueType::Neno, "tafuta") => ValueType::Chaguo(Box::new(ValueType::Namba)),
                        (ValueType::Jozi(k, v), "clona") => ValueType::Jozi(k.clone(), v.clone()),
                        (ValueType::Jozi(k, _), "kwanza") => *k,
                        (ValueType::Jozi(_, v), "pili") => *v,
                        (ValueType::Orodha(ref t), "clona") => ValueType::Orodha(t.clone()),
                        (ValueType::Orodha(_), "urefu") => ValueType::Namba,
                        (ValueType::Orodha(_), "ongeza") => ValueType::Tupu,
                        (ValueType::Orodha(_), "ingiza") => ValueType::Tupu,
                        (ValueType::Orodha(ref t), "ondoa") => ValueType::Chaguo(t.clone()),
                        (ValueType::Orodha(_), "kila_mmoja") => ValueType::Tupu,
                        (ValueType::Kamusi(ref k, ref v), "clona") => ValueType::Kamusi(k.clone(), v.clone()),
                        (ValueType::Kamusi(_, _), "idadi") => ValueType::Namba,
                        (ValueType::Kamusi(_, ref v), "pata") => ValueType::Chaguo(v.clone()),
                        (ValueType::Kamusi(_, _), "ingiza" | "weka_key") => ValueType::Tupu,
                        (ValueType::Kamusi(_, _), "vipo") => ValueType::Ukweli,
                        (ValueType::Kamusi(ref k, _), "funguo") => ValueType::Orodha(k.clone()),
                        (ValueType::Tokeo(ref t, _), "angu") => *t.clone(),
                        (ValueType::Tokeo(_, _), "ni_kosa" | "ni_sawa") => ValueType::Ukweli,
                        (ValueType::Tokeo(_, ref e), "kosa") => *e.clone(),
                        (ValueType::Chaguo(ref t), "angu" | "hakikisha") => *t.clone(),
                        (ValueType::Chaguo(_), "ni_po" | "ni_tupu") => ValueType::Ukweli,
                        (ValueType::KashaGC(ref t), "pata") => *t.clone(),
                        (ValueType::KashaGC(_), "weka") => ValueType::Tupu,
                        (ValueType::KashaGC(_), "idadi") => ValueType::Namba,
                        (ValueType::KashaGC(ref t), "shirikisha") => ValueType::KashaGC(t.clone()),
                        _ => ValueType::Unknown,
                    };
                }
                if !is_enum && !_is_struct {
                    self.errors.push(
                        Diagnostic::new(
                            "SEM104",
                            format!("njia inaweza tu kuwa juu ya umbo au jenum, si '{}'", receiver_ty_name),
                        )
                        .with_stage("semantiki")
                        .with_span(*line, 1),
                    );
                    return ValueType::Unknown;
                }

                let mut method_decl = None;
                for imp in &self.module.impls {
                    if imp.target == receiver_ty_name {
                        for func in &imp.body {
                            if func.name == *method_name {
                                method_decl = Some(func);
                                break;
                            }
                        }
                    }
                    if method_decl.is_some() {
                        break;
                    }
                }

                let Some(func) = method_decl else {
                    // Check for built-in methods on Tokeo and Chaguo before emitting SEM040
                    if is_enum {
                        let builtin_ret = match (receiver_ty_name.as_str(), method_name.as_str()) {
                            ("Tokeo", "ni_kosa" | "ni_sawa") => Some(ValueType::Ukweli),
                            ("Tokeo", "kosa" | "angu") => {
                                for arg in args {
                                    let _ = self.check_expr(arg, scopes, UseMode::Move);
                                }
                                Some(ValueType::TypeVar("T".to_string()))
                            }
                            ("Chaguo", "ni_po" | "ni_tupu") => Some(ValueType::Ukweli),
                            ("Chaguo", "angu" | "hakikisha") => {
                                for arg in args {
                                    let _ = self.check_expr(arg, scopes, UseMode::Move);
                                }
                                Some(ValueType::TypeVar("T".to_string()))
                            }
                            _ => None,
                        };
                        if let Some(ret) = builtin_ret {
                            return ret;
                        }
                    }
                    self.errors.push(
                        Diagnostic::new(
                            "SEM040",
                            format!("njia '{}' haipo kwa '{}'", method_name, receiver_ty_name),
                        )
                        .with_stage("semantiki")
                        .with_span(*line, 1),
                    );
                    return ValueType::Unknown;
                };

                // Validate arity (excluding self)
                if args.len() != func.params.len() - 1 {
                    self.errors.push(
                        Diagnostic::new(
                            "SEM041",
                            format!(
                                "njia '{}' inahitaji hoja {} (umetoa {})",
                                method_name,
                                func.params.len() - 1,
                                args.len()
                            ),
                        )
                        .with_stage("semantiki")
                        .with_span(*line, 1),
                    );
                    return self.type_from_decl(&func.return_type.name);
                }

                // Type check arguments
                for (i, arg) in args.iter().enumerate() {
                    let arg_ty = self.check_expr(arg, scopes, UseMode::Move);
                    let param_ty = self.type_from_decl(&func.params[i + 1].ty.name);
                    self.check_type_compatibility(
                        &param_ty,
                        &arg_ty,
                        "SEM042",
                        format!(
                            "hoja ya {} kwa '{}' haitalingana: inahitaji {}, imepata {}",
                            i + 1,
                            method_name,
                            param_ty,
                            arg_ty
                        ),
                        Span { line: *line, column: 1 },
                    );
                }

                self.type_from_decl(&func.return_type.name)
            }
            Expr::StructLiteral {
                struct_name,
                fields,
                line,
                ..
            } => {
                let Some(st) = self.module.structs.iter().find(|s| s.name == *struct_name) else {
                    self.errors.push(
                        Diagnostic::new("SEM093", format!("umbo haijulikani: {}", struct_name))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                    return ValueType::Unknown;
                };
                let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
                for (fname, fexpr) in fields {
                    if !st.fields.iter().any(|(n, _)| n == fname) {
                        self.errors.push(
                            Diagnostic::new("SEM094", format!("umbo '{}' halina uga '{}'", struct_name, fname))
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                    } else if !seen.insert(fname) {
                        self.errors.push(
                            Diagnostic::new("SEM095", format!("uga '{}' limeorodheshwa mara mbili", fname))
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                    }
                    let ft = self.check_expr(fexpr, scopes, UseMode::Move);
                    if let Some((_, Some(ref fty))) = st.fields.iter().find(|(n, _)| n == fname) {
                        let want = self.type_from_decl(&fty.name);
                        if !self.compatible(&want, &ft) {
                            self.errors.push(
                                Diagnostic::new("SEM096", format!("aina ya uga '{}' hailingani na thamani iliyotolewa", fname))
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                    }
                }
                for (fname, _) in &st.fields {
                    if !fields.iter().any(|(n, _)| n == fname) {
                        self.errors.push(
                            Diagnostic::new("SEM097", format!("umbo '{}' linahitaji uga '{}'", struct_name, fname))
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                    }
                }
                ValueType::Struct(struct_name.clone())
            }
            Expr::EnumConstruct {
                enum_name,
                variant_name: _,
                data,
                line,
                ..
            } => {
                let _en = self.module.enums.iter().find(|e| e.name == *enum_name);
                if _en.is_none() {
                    self.errors.push(
                        Diagnostic::new("SEM098", format!("jenum haijulikani: {}", enum_name))
                            .with_stage("semantiki")
                            .with_span(*line, 1),
                    );
                    return ValueType::Unknown;
                }
                if let Some(d) = data {
                    let _ = self.check_expr(d, scopes, UseMode::Move);
                }
                ValueType::Struct(enum_name.clone())
            }
            Expr::Index { base, index, line } => {
                let base_ty = self.check_expr(base, scopes, UseMode::BorrowImm);
                let idx_ty = self.check_expr(index, scopes, UseMode::Move);
                match &base_ty {
                    ValueType::Orodha(inner) => {
                        if idx_ty != ValueType::Namba && idx_ty != ValueType::Unknown {
                            self.errors.push(
                                Diagnostic::new("SEM102", "fahirisi inahitaji Namba")
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                        }
                        ValueType::Tokeo(
                            inner.clone(),
                            Box::new(ValueType::Struct("KosaMipaka".to_string())),
                        )
                    }
                    ValueType::Kamusi(_, v) => *v.clone(),
                    _ => {
                        self.errors.push(
                            Diagnostic::new("SEM103", "fahirisi inahitaji Orodha au Kamusi")
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                        ValueType::Unknown
                    }
                }
            }
            Expr::FieldAccess { receiver, field, line, .. } => {
                let rec_ty = self.check_expr(receiver, scopes, UseMode::BorrowImm);
                match &rec_ty {
                    ValueType::Struct(name) => {
                        let Some(st) = self.module.structs.iter().find(|s| s.name == *name) else {
                            return ValueType::Unknown;
                        };
                        if !st.fields.iter().any(|(n, _)| n == field) {
                            self.errors.push(
                                Diagnostic::new("SEM098", format!("umbo '{}' halina uga '{}'", name, field))
                                    .with_stage("semantiki")
                                    .with_span(*line, 1),
                            );
                            ValueType::Unknown
                        } else if let Some((_, Some(ref fty))) = st.fields.iter().find(|(n, _)| n == field) {
                            self.type_from_decl(&fty.name)
                        } else {
                            ValueType::Unknown
                        }
                    }
                    _ => {
                        self.errors.push(
                            Diagnostic::new("SEM099", "uga unahitaji kitu cha aina ya umbo")
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                        ValueType::Unknown
                    }
                }
            }
            Expr::Propagate { expr, line } => {
                let t = self.check_expr(expr, scopes, UseMode::Move);
                let idents = Self::collect_idents_from_expr(expr);
                self.mark_tokeo_consumed(scopes, &idents);
                match t {
                    ValueType::Tokeo(ok, _) => *ok,
                    ValueType::Chaguo(inner) => *inner,
                    _ => {
                        self.errors.push(
                            Diagnostic::new("SEM039", "? inahitaji Tokeo/Chaguo")
                                .with_stage("semantiki")
                                .with_span(*line, 1),
                        );
                        ValueType::Unknown
                    }
                }
            }
            Expr::List { elements, line } => {
                let mut ty = ValueType::Unknown;
                for e in elements {
                    let ety = self.check_expr(e, scopes, UseMode::Move);
                    if matches!(ty, ValueType::Unknown) {
                        ty = ety;
                    } else if !self.compatible(&ty, &ety) {
                        self.errors.push(Diagnostic::new("SEM099", "orodha inahitaji aina moja ya vipengele")
                            .with_stage("semantiki")
                            .with_span(*line, 1));
                    }
                }
                ValueType::Orodha(Box::new(ty))
            }
            Expr::Map { entries, line } => {
                let mut kty = ValueType::Unknown;
                let mut vty = ValueType::Unknown;
                for (k, v) in entries {
                    let kety = self.check_expr(k, scopes, UseMode::Move);
                    let vety = self.check_expr(v, scopes, UseMode::Move);
                    if matches!(kty, ValueType::Unknown) { kty = kety.clone(); }
                    if matches!(vty, ValueType::Unknown) { vty = vety.clone(); }
                    if !self.compatible(&kty, &kety) || !self.compatible(&vty, &vety) {
                        self.errors.push(Diagnostic::new("SEM100", "kamusi inahitaji aina moja ya ufunguo na thamani")
                            .with_stage("semantiki")
                            .with_span(*line, 1));
                    }
                }
                ValueType::Kamusi(Box::new(kty), Box::new(vty))
            }
        }
    }

    fn use_ident(
        &mut self,
        name: &str,
        scopes: &mut [HashMap<String, Binding>],
        mode: UseMode,
    ) -> ValueType {
        if matches!(
            name,
            "Ukomo" | "Siyo_Namba" | "INF" | "NAN" | "PI" | "E" | "PHI" | "TAU" | "LN2" | "LN10"
                | "LOG2E" | "LOG10E" | "KIPEUO1_2" | "KIPEUO2" | "KIPEUO3" | "KIPEUO5" | "EPSILON"
                | "SEKUNDE_KWA_SIKU" | "MWANZO_WA_ZAMANI"
        ) {
            return ValueType::Namba;
        }
        if matches!(name, "TOLEO" | "JINA_OS" | "NJIA_SEPARATOR") {
            return ValueType::Neno;
        }
        if matches!(name, "KWELI" | "SIYO_KWELI") {
            return ValueType::Ukweli;
        }
        if name == "TUPU" {
            return ValueType::Tupu;
        }
        if let Some(ty) = self.extern_constants.get(name) {
            return ty.clone();
        }
        if let Some(ty) = self.local_constants.get(name) {
            return ty.clone();
        }

        for scope in scopes.iter_mut().rev() {
            if let Some(b) = scope.get_mut(name) {
                if let Some(set) = self.dropped_vars.last() {
                    if set.contains(name) {
                        self.errors.push(
                            Diagnostic::new("SEM028", format!("'{name}' tayari tupwa — haipaswi kutumiwa tena"))
                                .with_stage("semantiki")
                                .with_span(b.created_at.line, b.created_at.column),
                        );
                        return b.ty.clone();
                    }
                }
                if b.moved {
                    let map = ContextMap {
                        symbol: name.to_string(),
                        created_at: Some(b.created_at.clone()),
                        moved_at: b.moved_at.clone(),
                        borrowed_at: b.borrowed_at.clone(),
                        dropped_at: b.dropped_at.clone(),
                    };
                    self.errors.push(
                        Diagnostic::new("SEM040", format!("matumizi baada ya kuhamisha: {name}"))
                            .with_stage("semantiki")
                            .with_context_map(map),
                    );
                    return b.ty.clone();
                }

                match mode {
                    UseMode::BorrowImm => {
                        if b.mut_borrowed {
                            self.errors.push(
                                Diagnostic::new("SEM041", format!("haiwezi azima wakati azima_tenda ipo: {name}"))
                                    .with_stage("semantiki"),
                            );
                        }
                        b.imm_borrows += 1;
                        b.borrowed_at.push(Span {
                            line: b.created_at.line,
                            column: b.created_at.column,
                        });
                        return b.ty.clone();
                    }
                    UseMode::BorrowMut => {
                        if !b.mutable {
                            self.errors.push(
                                Diagnostic::new("SEM042", format!("azima_tenda inahitaji jina linalobadilika: {name}"))
                                    .with_stage("semantiki"),
                            );
                        }
                        if b.mut_borrowed || b.imm_borrows > 0 {
                            self.errors.push(
                                Diagnostic::new("SEM043", format!("migongano ya kuazima kwa: {name}"))
                                    .with_stage("semantiki"),
                            );
                        }
                        b.mut_borrowed = true;
                        b.borrowed_at.push(Span {
                            line: b.created_at.line,
                            column: b.created_at.column,
                        });
                        return b.ty.clone();
                    }
                    UseMode::Move => {
                        if !self.is_copy_type(&b.ty) && b.mut_borrowed {
                            // TODO(Phase III): Re-enable immutable borrow check once lifetimes are modeled.
                            self.errors.push(
                                Diagnostic::new("SEM044", format!("haiwezi kuhamisha wakati azima_tenda ipo: {name}"))
                                    .with_stage("semantiki"),
                            );
                        }
                        if !self.is_copy_type(&b.ty) {
                            b.moved = true;
                            b.moved_at = Some(Span {
                                line: b.created_at.line,
                                column: b.created_at.column,
                            });
                        }
                        return b.ty.clone();
                    }
                    UseMode::Return => {
                        if !self.is_copy_type(&b.ty) && b.mut_borrowed {
                            self.errors.push(
                                Diagnostic::new("SEM044", format!("haiwezi rejesha wakati azima_tenda ipo: {name}"))
                                    .with_stage("semantiki"),
                            );
                        }
                        if !self.is_copy_type(&b.ty) {
                            b.moved = true;
                            b.moved_at = Some(Span {
                                line: b.created_at.line,
                                column: b.created_at.column,
                            });
                        }
                        return b.ty.clone();
                    }
                }
            }
        }

        self.errors.push(
            Diagnostic::new("SEM045", format!("jina halijulikani: {name}")).with_stage("semantiki"),
        );
        ValueType::Unknown
    }

    // TODO(Phase III): Herufi and Tupu should also be Copy. Neno, Orodha, Kamusi, and Struct
    // are non-Copy but are currently treated as moved only at the semantic level — the evaluator
    // clones them unconditionally, so move semantics are not enforced at runtime.
    fn is_copy_type(&self, ty: &ValueType) -> bool {
        matches!(
            ty,
            ValueType::Namba
                | ValueType::Ukweli
                | ValueType::Tupu
                | ValueType::Hamna
                | ValueType::Herufi
                | ValueType::Wakati
                | ValueType::Anuani
        )
    }

    // TODO(Phase II): compatible() is a simple structural equality check. Missing cases:
    // - Trait bounds: `T: Sifa` constraints are not verified
    // - Generic instantiation: Orodha<Namba> vs Orodha<Neno> are not distinguished (both Unknown)
    // - Coercions: &T -> &Tupu, Struct -> Sifa (trait object) upcasting
    fn compatible(&self, a: &ValueType, b: &ValueType) -> bool {
        // Type variables are compatible with anything (unbound generics)
        if matches!(a, ValueType::TypeVar(_)) || matches!(b, ValueType::TypeVar(_)) {
            return true;
        }
        // Unknown acts as a wildcard in Phase I
        if matches!(a, ValueType::Unknown) || matches!(b, ValueType::Unknown) {
            return true;
        }
        match (a, b) {
            (ValueType::Orodha(a1), ValueType::Orodha(b1)) => self.compatible(a1, b1),
            (ValueType::Kamusi(ak, av), ValueType::Kamusi(bk, bv)) => {
                self.compatible(ak, bk) && self.compatible(av, bv)
            }
            (ValueType::Jozi(a1, a2), ValueType::Jozi(b1, b2)) => {
                self.compatible(a1, b1) && self.compatible(a2, b2)
            }
            (ValueType::Chaguo(a1), ValueType::Chaguo(b1)) => self.compatible(a1, b1),
            (ValueType::Tokeo(a1, a2), ValueType::Tokeo(b1, b2)) => {
                self.compatible(a1, b1) && self.compatible(a2, b2)
            }
            (ValueType::Rejeo(a1, am), ValueType::Rejeo(b1, bm)) => {
                am == bm && self.compatible(a1, b1)
            }
            _ => a == b,
        }
    }

    /// Checks compatibility and emits a diagnostic if they are incompatible
    /// or if one of the types is `Unknown` (inference failure).
    fn check_type_compatibility(
        &mut self,
        expected: &ValueType,
        actual: &ValueType,
        error_code: &'static str,
        message: String,
        span: Span,
    ) -> bool {
        // Type variables are always compatible (unbound generics)
        if matches!(expected, ValueType::TypeVar(_)) || matches!(actual, ValueType::TypeVar(_)) {
            return true;
        }

        if matches!(expected, ValueType::Unknown) || matches!(actual, ValueType::Unknown) {
            // Assume compatibility to avoid cascading errors — this is the correct behavior for
            // unresolved-generic results (e.g. `kamusi_tupu().pata(k).angu(default)`; msingi's
            // generic constructors don't propagate real type params, so their results are
            // ValueType::Unknown until narrowed). This used to also push a "SEM-INF" diagnostic
            // here, but a pushed diagnostic still fails the whole `semantic_check_with_env` call
            // (there is no non-blocking/warning severity in this Vec<Diagnostic> design) —
            // directly contradicting the "assume compatible" comment and rejecting valid
            // programs that legitimately hit this path. Don't record one; only bail (return
            // false) on a genuine, both-sides-concrete mismatch below.
            return true;
        }

        if !self.compatible(expected, actual) {
            self.errors.push(
                Diagnostic::new(error_code, message)
                    .with_stage("semantiki")
                    .with_span(span.line, span.column),
            );
            return false;
        }
        true
    }

    fn type_from_decl(&self, t: &str) -> ValueType {
        let s = t.trim();
        if self.module.structs.iter().any(|st| st.name == s) {
            return ValueType::Struct(s.to_string());
        }
        if self.module.enums.iter().any(|e| e.name == s) {
            return ValueType::Struct(s.to_string());
        }
        parse_value_type(t)
    }
}

pub(crate) fn run_semantic_check(
    module: &Module,
    require_main: bool,
    extern_functions: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
) -> Vec<Diagnostic> {
    run_semantic_check_with_modules(module, require_main, extern_functions, extern_constants, HashSet::new())
}

/// Like `run_semantic_check`, but `resolved_modules` names additional `leta` targets to accept
/// beyond the fixed builtin-module whitelist (SEM007) — e.g. user/path-dependency modules that a
/// project resolver has already confirmed exist. Callers with no resolver (LSP single-file
/// checks, standalone `semantic_check`) pass an empty set, preserving prior behavior.
pub(crate) fn run_semantic_check_with_modules(
    module: &Module,
    require_main: bool,
    extern_functions: HashMap<String, FnContract>,
    extern_constants: HashMap<String, ValueType>,
    resolved_modules: HashSet<String>,
) -> Vec<Diagnostic> {
    let mut a = Analyzer::new(module, require_main, extern_functions, extern_constants, resolved_modules);
    a.run();
    std::mem::take(&mut a.errors)
}
