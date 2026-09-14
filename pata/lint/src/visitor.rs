//! AST visitor trait for lint rules

use asili_parser::{Module, Stmt, Expr};

/// Visitor trait for traversing AST
pub trait Visitor {
    fn visit_module(&mut self, module: &Module) {
        for func in &module.functions {
            self.visit_block(&func.body);
        }
    }

    fn visit_block(&mut self, block: &asili_parser::Block) {
        for stmt in &block.statements {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, _stmt: &Stmt) {}

    fn visit_expr(&mut self, _expr: &Expr) {}
}
