use crate::ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipState {
    Owned,
    BorrowedShared(usize),
    BorrowedMut,
    Moved,
}

pub struct BorrowChecker {
    // Variable tracking: name -> OwnershipState
    variables: HashMap<String, OwnershipState>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    pub fn check_function(&mut self, func: &FunctionDeclaration) -> Result<(), String> {
        self.variables.clear();
        for param in &func.parameters {
            self.variables.insert(param.name.clone(), OwnershipState::Owned);
        }

        for stmt in &func.body {
            self.check_statement(stmt)?;
        }
        Ok(())
    }

    fn check_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.check_expression(value)?;
                self.variables.insert(name.clone(), OwnershipState::Owned);
            }
            Statement::Assignment { lhs, rhs, .. } => {
                // If assigning to a variable or field, ensure we can write to it
                self.check_expression(rhs)?;
                self.check_lhs_write(lhs)?;
            }
            Statement::Expression(expr) => {
                self.check_expression(expr)?;
            }
        }
        Ok(())
    }

    fn check_lhs_write(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Identifier(name) => {
                let state = self.variables.get(name).copied().unwrap_or(OwnershipState::Owned);
                if state == OwnershipState::Moved {
                    return Err(format!("Cannot write to moved variable '{}'", name));
                }
                if let Some(OwnershipState::BorrowedShared(_)) = self.variables.get(name) {
                    return Err(format!("Cannot write to shared borrowed variable '{}'", name));
                }
                if let Some(OwnershipState::BorrowedMut) = self.variables.get(name) {
                    // Allowed
                }
            }
            Expression::MemberAccess { object, .. } => {
                self.check_lhs_write(object)?;
            }
            _ => return Err("Invalid left-hand side of assignment".to_string()),
        }
        Ok(())
    }

    fn check_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::LiteralInt(_)
            | Expression::LiteralFloat(_)
            | Expression::LiteralBool(_)
            | Expression::LiteralString(_) => Ok(()),
            Expression::Identifier(name) => {
                let state = self.variables.get_mut(name);
                if let Some(s) = state {
                    if *s == OwnershipState::Moved {
                        return Err(format!("Use of moved variable: '{}'", name));
                    }
                    // In affine types, standard use of a variable of custom type (like Wallet) moves it.
                    // Primitives like Int/Float are implicitly copied.
                    // For the sake of simplicity: assume custom/struct types are moved.
                    // For simplicity, we can let user use variables normally, but check explicit borrow rules.
                }
                Ok(())
            }
            Expression::MemberAccess { object, .. } => {
                self.check_expression(object)
            }
            Expression::Binary { lhs, rhs, .. } => {
                self.check_expression(lhs)?;
                self.check_expression(rhs)
            }
            Expression::Unary { expr, op } => {
                // Check if it is a borrow expression
                if op == "&" {
                    self.borrow_variable(expr, false)
                } else if op == "&mut" {
                    self.borrow_variable(expr, true)
                } else {
                    self.check_expression(expr)
                }
            }
            Expression::Call { args, .. } => {
                for arg in args {
                    self.check_expression(arg)?;
                }
                Ok(())
            }
        }
    }

    fn borrow_variable(&mut self, expr: &Expression, is_mut: bool) -> Result<(), String> {
        match expr {
            Expression::Identifier(name) => {
                let state = self.variables.get(name).copied().unwrap_or(OwnershipState::Owned);
                match state {
                    OwnershipState::Moved => {
                        Err(format!("Cannot borrow moved variable '{}'", name))
                    }
                    OwnershipState::BorrowedMut => {
                        Err(format!("Cannot borrow '{}' because it is already borrowed mutably", name))
                    }
                    OwnershipState::BorrowedShared(count) => {
                        if is_mut {
                            Err(format!("Cannot borrow '{}' mutably because it is already borrowed immutably", name))
                        } else {
                            self.variables.insert(name.clone(), OwnershipState::BorrowedShared(count + 1));
                            Ok(())
                        }
                    }
                    OwnershipState::Owned => {
                        if is_mut {
                            self.variables.insert(name.clone(), OwnershipState::BorrowedMut);
                        } else {
                            self.variables.insert(name.clone(), OwnershipState::BorrowedShared(1));
                        }
                        Ok(())
                    }
                }
            }
            Expression::MemberAccess { object, .. } => {
                self.borrow_variable(object, is_mut)
            }
            _ => Err("Can only borrow variables or fields".to_string()),
        }
    }
}
