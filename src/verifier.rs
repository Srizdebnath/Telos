use crate::ast::*;
use z3::ast::{Ast, Bool, Int};
use z3::{Context, Solver};
use std::collections::HashMap;

pub struct Verifier<'ctx> {
    ctx: &'ctx Context,
    // Maps a flat variable/field name (e.g. "source.balance") to its SSA history (list of Z3 Int/Bool variables)
    // We only need to support Int and Bool in SMT for Stage 0.
    variables: HashMap<String, Vec<z3::ast::Dynamic<'ctx>>>,
    structs: HashMap<String, StructDeclaration>,
}

impl<'ctx> Verifier<'ctx> {
    pub fn new(ctx: &'ctx Context, structs: &[StructDeclaration]) -> Self {
        let mut struct_map = HashMap::new();
        for s in structs {
            struct_map.insert(s.name.clone(), s.clone());
        }
        Self {
            ctx,
            variables: HashMap::new(),
            structs: struct_map,
        }
    }

    // Helper to get the current (latest) version of a variable or member path
    fn get_current_var(&self, path: &str) -> Option<&z3::ast::Dynamic<'ctx>> {
        self.variables.get(path).and_then(|v| v.last())
    }

    // Helper to get the initial (version 0) of a variable or member path
    fn get_initial_var(&self, path: &str) -> Option<&z3::ast::Dynamic<'ctx>> {
        self.variables.get(path).and_then(|v| v.first())
    }

    // Add a new SSA version for a path
    fn push_var(&mut self, path: String, var: z3::ast::Dynamic<'ctx>) {
        self.variables.entry(path).or_default().push(var);
    }

    fn init_parameter(&mut self, param: &Parameter) {
        match &param.ty {
            Type::Int => {
                let var = Int::new_const(self.ctx, param.name.clone());
                self.push_var(param.name.clone(), z3::ast::Dynamic::from(var));
            }
            Type::Bool => {
                let var = Bool::new_const(self.ctx, param.name.clone());
                self.push_var(param.name.clone(), z3::ast::Dynamic::from(var));
            }
            Type::Custom(struct_name) => {
                if let Some(s_decl) = self.structs.get(struct_name).cloned() {
                    for field in &s_decl.fields {
                        let path = format!("{}.{}", param.name, field.name);
                        match &field.ty {
                            Type::Int => {
                                let var = Int::new_const(self.ctx, path.clone());
                                self.push_var(path, z3::ast::Dynamic::from(var));
                            }
                            Type::Bool => {
                                let var = Bool::new_const(self.ctx, path.clone());
                                self.push_var(path, z3::ast::Dynamic::from(var));
                            }
                            _ => {} // String/Float etc ignored in SMT for Stage 0
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn eval_expr(&self, expr: &Expression, use_initial: bool) -> Result<z3::ast::Dynamic<'ctx>, String> {
        match expr {
            Expression::LiteralInt(v) => {
                Ok(z3::ast::Dynamic::from(Int::from_i64(self.ctx, *v)))
            }
            Expression::LiteralBool(v) => {
                Ok(z3::ast::Dynamic::from(Bool::from_bool(self.ctx, *v)))
            }
            Expression::Identifier(name) => {
                let path = name.as_str();
                let var = if use_initial {
                    self.get_initial_var(path)
                } else {
                    self.get_current_var(path)
                };
                var.cloned().ok_or_else(|| format!("Variable '{}' not defined or not supported in verifier", name))
            }
            Expression::MemberAccess { object, member } => {
                // Flatten member access, e.g. source.balance
                if let Expression::Identifier(ref obj_name) = **object {
                    let path = format!("{}.{}", obj_name, member);
                    let var = if use_initial {
                        self.get_initial_var(&path)
                    } else {
                        self.get_current_var(&path)
                    };
                    var.cloned().ok_or_else(|| format!("Member path '{}' not found or not supported", path))
                } else {
                    // Nested member access
                    let mut path_parts = Vec::new();
                    let mut curr = expr;
                    while let Expression::MemberAccess { object, member } = curr {
                        path_parts.push(member.as_str());
                        curr = object;
                    }
                    if let Expression::Identifier(ref obj_name) = *curr {
                        path_parts.push(obj_name.as_str());
                        path_parts.reverse();
                        let path = path_parts.join(".");
                        let var = if use_initial {
                            self.get_initial_var(&path)
                        } else {
                            self.get_current_var(&path)
                        };
                        var.cloned().ok_or_else(|| format!("Member path '{}' not found or not supported", path))
                    } else {
                        Err("Complex member access object is not supported".to_string())
                    }
                }
            }
            Expression::Unary { op, expr } => {
                let val = self.eval_expr(expr, use_initial)?;
                if op == "!" {
                    let b = val.as_bool().ok_or("Negation expects a boolean expression")?;
                    Ok(z3::ast::Dynamic::from(b.not()))
                } else if op == "-" {
                    let i = val.as_int().ok_or("Arithmetic negation expects an integer expression")?;
                    Ok(z3::ast::Dynamic::from(i.unary_minus()))
                } else {
                    Err(format!("Unsupported unary operator in SMT: {}", op))
                }
            }
            Expression::Call { function, args } => {
                if function == "initial" && args.len() == 1 {
                    // Evaluate argument using initial values!
                    self.eval_expr(&args[0], true)
                } else {
                    Err(format!("Unsupported function call in SMT: {}", function))
                }
            }
            Expression::Binary { lhs, op, rhs } => {
                let l_val = self.eval_expr(lhs, use_initial)?;
                let r_val = self.eval_expr(rhs, use_initial)?;

                match op {
                    BinaryOp::Add => {
                        let l = l_val.as_int().ok_or("Add expects integers")?;
                        let r = r_val.as_int().ok_or("Add expects integers")?;
                        Ok(z3::ast::Dynamic::from(Int::add(self.ctx, &[&l, &r])))
                    }
                    BinaryOp::Sub => {
                        let l = l_val.as_int().ok_or("Sub expects integers")?;
                        let r = r_val.as_int().ok_or("Sub expects integers")?;
                        Ok(z3::ast::Dynamic::from(Int::sub(self.ctx, &[&l, &r])))
                    }
                    BinaryOp::Mul => {
                        let l = l_val.as_int().ok_or("Mul expects integers")?;
                        let r = r_val.as_int().ok_or("Mul expects integers")?;
                        Ok(z3::ast::Dynamic::from(Int::mul(self.ctx, &[&l, &r])))
                    }
                    BinaryOp::Div => {
                        let l = l_val.as_int().ok_or("Div expects integers")?;
                        let r = r_val.as_int().ok_or("Div expects integers")?;
                        Ok(z3::ast::Dynamic::from(l.div(&r)))
                    }
                    BinaryOp::Eq => {
                        Ok(z3::ast::Dynamic::from(l_val._eq(&r_val)))
                    }
                    BinaryOp::Ne => {
                        Ok(z3::ast::Dynamic::from(l_val._eq(&r_val).not()))
                    }
                    BinaryOp::Lt => {
                        let l = l_val.as_int().ok_or("< expects integers")?;
                        let r = r_val.as_int().ok_or("< expects integers")?;
                        Ok(z3::ast::Dynamic::from(l.lt(&r)))
                    }
                    BinaryOp::Le => {
                        let l = l_val.as_int().ok_or("<= expects integers")?;
                        let r = r_val.as_int().ok_or("<= expects integers")?;
                        Ok(z3::ast::Dynamic::from(l.le(&r)))
                    }
                    BinaryOp::Gt => {
                        let l = l_val.as_int().ok_or("> expects integers")?;
                        let r = r_val.as_int().ok_or("> expects integers")?;
                        Ok(z3::ast::Dynamic::from(l.gt(&r)))
                    }
                    BinaryOp::Ge => {
                        let l = l_val.as_int().ok_or(">= expects integers")?;
                        let r = r_val.as_int().ok_or(">= expects integers")?;
                        Ok(z3::ast::Dynamic::from(l.ge(&r)))
                    }
                    BinaryOp::And => {
                        let l = l_val.as_bool().ok_or("&& expects booleans")?;
                        let r = r_val.as_bool().ok_or("&& expects booleans")?;
                        Ok(z3::ast::Dynamic::from(Bool::and(self.ctx, &[&l, &r])))
                    }
                    BinaryOp::Or => {
                        let l = l_val.as_bool().ok_or("|| expects booleans")?;
                        let r = r_val.as_bool().ok_or("|| expects booleans")?;
                        Ok(z3::ast::Dynamic::from(Bool::or(self.ctx, &[&l, &r])))
                    }
                    BinaryOp::Implies => {
                        let l = l_val.as_bool().ok_or("-> expects booleans")?;
                        let r = r_val.as_bool().ok_or("-> expects booleans")?;
                        Ok(z3::ast::Dynamic::from(l.implies(&r)))
                    }
                }
            }
            _ => Err("Expression type not supported in verification".to_string()),
        }
    }

    fn resolve_path(&self, expr: &Expression) -> Result<String, String> {
        match expr {
            Expression::Identifier(name) => Ok(name.clone()),
            Expression::MemberAccess { object, member } => {
                let obj_path = self.resolve_path(object)?;
                Ok(format!("{}.{}", obj_path, member))
            }
            _ => Err("LHS is not a valid assignable variable or field path".to_string()),
        }
    }

    pub fn verify_function(&mut self, func: &FunctionDeclaration, intent: &IntentDeclaration) -> Result<(), String> {
        let solver = Solver::new(self.ctx);

        // 1. Initialize parameter variables (version 0)
        for param in &func.parameters {
            self.init_parameter(param);
        }

        // 2. Assert preconditions
        for prec in &intent.preconditions {
            let expr_z3 = self.eval_expr(prec, false)?;
            let b = expr_z3.as_bool().ok_or("Preconditions must be boolean expressions")?;
            solver.assert(&b);
        }

        // 3. Process function body statements (SSA generation)
        for stmt in &func.body {
            match stmt {
                Statement::Let { name, value, .. } => {
                    let val_z3 = self.eval_expr(value, false)?;
                    let var_name = format!("{}_{}", name, self.variables.get(name).map(|v| v.len()).unwrap_or(0));
                    if let Some(int_val) = val_z3.as_int() {
                        let new_var = Int::new_const(self.ctx, var_name);
                        solver.assert(&new_var._eq(&int_val));
                        self.push_var(name.clone(), z3::ast::Dynamic::from(new_var));
                    } else if let Some(bool_val) = val_z3.as_bool() {
                        let new_var = Bool::new_const(self.ctx, var_name);
                        solver.assert(&new_var._eq(&bool_val));
                        self.push_var(name.clone(), z3::ast::Dynamic::from(new_var));
                    } else {
                        return Err(format!("Unsupported type for variable binding '{}'", name));
                    }
                }
                Statement::Assignment { lhs, op, rhs } => {
                    let path = self.resolve_path(lhs)?;
                    let rhs_z3 = self.eval_expr(rhs, false)?;
                    let current_lhs = self.get_current_var(&path).cloned()
                        .ok_or_else(|| format!("Assigning to undefined variable/path: {}", path))?;

                    let val_to_assign = match op {
                        Some(BinaryOp::Add) => {
                            let l = current_lhs.as_int().ok_or("+= expects integer lhs")?;
                            let r = rhs_z3.as_int().ok_or("+= expects integer rhs")?;
                            z3::ast::Dynamic::from(Int::add(self.ctx, &[&l, &r]))
                        }
                        Some(BinaryOp::Sub) => {
                            let l = current_lhs.as_int().ok_or("-= expects integer lhs")?;
                            let r = rhs_z3.as_int().ok_or("-= expects integer rhs")?;
                            z3::ast::Dynamic::from(Int::sub(self.ctx, &[&l, &r]))
                        }
                        None => rhs_z3,
                        _ => return Err("Unsupported assignment operator in verifier".to_string()),
                    };

                    let new_var_name = format!("{}_{}", path.replace(".", "_"), self.variables.get(&path).map(|v| v.len()).unwrap_or(0));
                    if current_lhs.as_int().is_some() {
                        let new_var = Int::new_const(self.ctx, new_var_name);
                        solver.assert(&new_var._eq(&val_to_assign.as_int().unwrap()));
                        self.push_var(path, z3::ast::Dynamic::from(new_var));
                    } else if current_lhs.as_bool().is_some() {
                        let new_var = Bool::new_const(self.ctx, new_var_name);
                        solver.assert(&new_var._eq(&val_to_assign.as_bool().unwrap()));
                        self.push_var(path, z3::ast::Dynamic::from(new_var));
                    } else {
                        return Err(format!("Unsupported assignment type for path '{}'", path));
                    }
                }
                Statement::Expression(expr) => {
                    // Expression statements usually don't affect verification unless they assert,
                    // but we check them for type safety/validity.
                    self.eval_expr(expr, false)?;
                }
            }
        }

        // 4. Evaluate postconditions and check if there's any state violating them
        // Postconditions should hold. We negate them to find counter-examples.
        // Fails if: Preconditions AND Body AND NOT Postconditions is Satisfiable.
        let mut post_assertions = Vec::new();
        for post in &intent.postconditions {
            let expr_z3 = self.eval_expr(post, false)?;
            let b = expr_z3.as_bool().ok_or("Postconditions must be boolean expressions")?;
            post_assertions.push(b);
        }

        if !post_assertions.is_empty() {
            let all_posts = Bool::and(self.ctx, &post_assertions.iter().collect::<Vec<_>>());
            let negated_posts = all_posts.not();
            solver.assert(&negated_posts);
        }

        // 5. Run SMT Solver check
        match solver.check() {
            z3::SatResult::Unsat => {
                // Unsat means there is NO model where preconditions hold, body executes, and postconditions fail.
                // Therefore, code is correct!
                Ok(())
            }
            z3::SatResult::Sat => {
                // Sat means a counter-example is found!
                let model = solver.get_model().unwrap();
                let mut counter_example = String::new();
                counter_example.push_str("Verification Failed! Counter-example found:\n");
                
                // Let's print out the initial parameter values
                for param in &func.parameters {
                    match &param.ty {
                        Type::Int | Type::Bool => {
                            if let Some(var) = self.get_initial_var(&param.name) {
                                if let Some(val) = model.eval(var, true) {
                                    counter_example.push_str(&format!("  {} = {}\n", param.name, val));
                                }
                            }
                        }
                        Type::Custom(struct_name) => {
                            if let Some(s_decl) = self.structs.get(struct_name) {
                                counter_example.push_str(&format!("  {} (struct {}):\n", param.name, struct_name));
                                for field in &s_decl.fields {
                                    let path = format!("{}.{}", param.name, field.name);
                                    if let Some(var) = self.get_initial_var(&path) {
                                        if let Some(val) = model.eval(var, true) {
                                            counter_example.push_str(&format!("    .{} = {}\n", field.name, val));
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                
                Err(counter_example)
            }
            z3::SatResult::Unknown => {
                Err("Verification Unknown: Z3 could not determine satisfiability".to_string())
            }
        }
    }
}
