use crate::ast::*;
use z3::ast::{Ast, Bool, Int, Real, BV, String as Z3String};
use z3::{Context, Solver};
use std::collections::HashMap;

pub struct Verifier<'ctx> {
    ctx: &'ctx Context,
    // Maps a flat variable/field name (e.g. "source.balance") to its SSA history (list of Z3 Int/Bool/etc. variables)
    variables: HashMap<String, Vec<z3::ast::Dynamic<'ctx>>>,
    structs: HashMap<String, StructDeclaration>,
    type_aliases: HashMap<String, Type>,
}

impl<'ctx> Verifier<'ctx> {
    pub fn new(ctx: &'ctx Context, structs: &[StructDeclaration], type_aliases: &HashMap<String, Type>) -> Self {
        let mut struct_map = HashMap::new();
        for s in structs {
            struct_map.insert(s.name.clone(), s.clone());
        }
        Self {
            ctx,
            variables: HashMap::new(),
            structs: struct_map,
            type_aliases: type_aliases.clone(),
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

    fn init_parameter(&mut self, solver: &Solver, param: &Parameter) -> Result<(), String> {
        self.init_var_with_type(solver, &param.name, &param.ty)
    }

    fn init_var_with_type(&mut self, solver: &Solver, path: &str, ty: &Type) -> Result<(), String> {
        match ty {
            Type::Int => {
                let var = Int::new_const(self.ctx, path);
                self.push_var(path.to_string(), z3::ast::Dynamic::from(var));
            }
            Type::Bool => {
                let var = Bool::new_const(self.ctx, path);
                self.push_var(path.to_string(), z3::ast::Dynamic::from(var));
            }
            Type::Byte => {
                let var = BV::new_const(self.ctx, path, 8);
                self.push_var(path.to_string(), z3::ast::Dynamic::from(var));
            }
            Type::Float => {
                let var = Real::new_const(self.ctx, path);
                self.push_var(path.to_string(), z3::ast::Dynamic::from(var));
            }
            Type::String => {
                let var = Z3String::new_const(self.ctx, path);
                self.push_var(path.to_string(), z3::ast::Dynamic::from(var));
            }
            Type::Refinement { base, value_name, predicate } => {
                self.init_var_with_type(solver, path, base)?;
                let var_z3 = self.get_current_var(path).cloned().unwrap();
                self.assert_refinement(solver, &var_z3, value_name, predicate)?;
            }
            Type::Custom(name) => {
                if let Some(aliased_ty) = self.type_aliases.get(name).cloned() {
                    self.init_var_with_type(solver, path, &aliased_ty)?;
                } else if let Some(s_decl) = self.structs.get(name).cloned() {
                    for field in &s_decl.fields {
                        let field_path = format!("{}.{}", path, field.name);
                        self.init_var_with_type(solver, &field_path, &field.ty)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn create_ssa_var(&self, ty: &Type, name: &str) -> Result<z3::ast::Dynamic<'ctx>, String> {
        match ty {
            Type::Int => Ok(z3::ast::Dynamic::from(Int::new_const(self.ctx, name))),
            Type::Bool => Ok(z3::ast::Dynamic::from(Bool::new_const(self.ctx, name))),
            Type::Byte => Ok(z3::ast::Dynamic::from(BV::new_const(self.ctx, name, 8))),
            Type::Float => Ok(z3::ast::Dynamic::from(Real::new_const(self.ctx, name))),
            Type::String => Ok(z3::ast::Dynamic::from(Z3String::new_const(self.ctx, name))),
            Type::Refinement { base, .. } => self.create_ssa_var(base, name),
            Type::Custom(struct_name) => {
                if let Some(aliased_ty) = self.type_aliases.get(struct_name) {
                    self.create_ssa_var(aliased_ty, name)
                } else {
                    Err(format!("Cannot create SSA variable of custom struct type {:?}", struct_name))
                }
            }
            _ => Err(format!("Cannot create SSA variable of type {:?}", ty)),
        }
    }

    fn create_dynamic_ssa_var(&self, val: &z3::ast::Dynamic<'ctx>, name: &str) -> Result<z3::ast::Dynamic<'ctx>, String> {
        if val.as_int().is_some() {
            Ok(z3::ast::Dynamic::from(Int::new_const(self.ctx, name)))
        } else if val.as_bool().is_some() {
            Ok(z3::ast::Dynamic::from(Bool::new_const(self.ctx, name)))
        } else if val.as_real().is_some() {
            Ok(z3::ast::Dynamic::from(Real::new_const(self.ctx, name)))
        } else if val.as_bv().is_some() {
            Ok(z3::ast::Dynamic::from(BV::new_const(self.ctx, name, 8)))
        } else if val.as_string().is_some() {
            Ok(z3::ast::Dynamic::from(Z3String::new_const(self.ctx, name)))
        } else {
            Err("Cannot determine sort of Z3 value for SSA instantiation".to_string())
        }
    }

    fn assert_refinement(
        &mut self,
        solver: &Solver,
        var_z3: &z3::ast::Dynamic<'ctx>,
        value_name: &str,
        predicate: &Expression,
    ) -> Result<(), String> {
        // Temporarily bind the variable value to value_name in scope
        self.push_var(value_name.to_string(), var_z3.clone());
        let pred_z3 = self.eval_expr(predicate, false)?;
        let b = pred_z3.as_bool().ok_or("Refinement predicate must be a boolean expression")?;
        solver.assert(&b);
        // Pop variables to clean up scope
        if let Some(hist) = self.variables.get_mut(value_name) {
            hist.pop();
        }
        Ok(())
    }

    fn assert_type_refinement(&mut self, solver: &Solver, path: &str, ty: &Type) -> Result<(), String> {
        match ty {
            Type::Refinement { base, value_name, predicate } => {
                let var_z3 = self.get_current_var(path).cloned()
                    .ok_or_else(|| format!("Variable '{}' not found for refinement check", path))?;
                self.assert_refinement(solver, &var_z3, value_name, predicate)?;
                self.assert_type_refinement(solver, path, base)?;
            }
            Type::Custom(name) => {
                if let Some(aliased_ty) = self.type_aliases.get(name).cloned() {
                    self.assert_type_refinement(solver, path, &aliased_ty)?;
                } else if let Some(s_decl) = self.structs.get(name).cloned() {
                    for field in &s_decl.fields {
                        let field_path = format!("{}.{}", path, field.name);
                        self.assert_type_refinement(solver, &field_path, &field.ty)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_path_type(&self, path: &str, func_params: &[Parameter]) -> Option<Type> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }
        let start_param = func_params.iter().find(|p| p.name == parts[0])?;
        let mut current_ty = start_param.ty.clone();

        for member in &parts[1..] {
            // Resolve aliases first
            while let Type::Custom(name) = &current_ty {
                if let Some(aliased_ty) = self.type_aliases.get(name) {
                    current_ty = aliased_ty.clone();
                } else {
                    break;
                }
            }
            match current_ty {
                Type::Custom(struct_name) => {
                    let s_decl = self.structs.get(&struct_name)?;
                    let field = s_decl.fields.iter().find(|f| f.name == *member)?;
                    current_ty = field.ty.clone();
                }
                Type::Refinement { base, .. } => {
                    let mut base_unwrapped = base.clone();
                    while let Type::Refinement { base: inner_base, .. } = *base_unwrapped {
                        base_unwrapped = inner_base;
                    }
                    while let Type::Custom(name) = &*base_unwrapped {
                        if let Some(aliased_ty) = self.type_aliases.get(name) {
                            base_unwrapped = Box::new(aliased_ty.clone());
                        } else {
                            break;
                        }
                    }
                    if let Type::Custom(struct_name) = *base_unwrapped {
                        let s_decl = self.structs.get(&struct_name)?;
                        let field = s_decl.fields.iter().find(|f| f.name == *member)?;
                        current_ty = field.ty.clone();
                    } else {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        // Final resolve for the result type
        while let Type::Custom(name) = &current_ty {
            if let Some(aliased_ty) = self.type_aliases.get(name) {
                current_ty = aliased_ty.clone();
            } else {
                break;
            }
        }
        Some(current_ty)
    }

    fn eval_expr(&self, expr: &Expression, use_initial: bool) -> Result<z3::ast::Dynamic<'ctx>, String> {
        match expr {
            Expression::LiteralInt(v) => {
                Ok(z3::ast::Dynamic::from(Int::from_i64(self.ctx, *v)))
            }
            Expression::LiteralBool(v) => {
                Ok(z3::ast::Dynamic::from(Bool::from_bool(self.ctx, *v)))
            }
            Expression::LiteralFloat(v) => {
                // Convert float to rational to prevent precision problems
                let num = (v * 1000000.0) as i64;
                let den = 1000000i64;
                Ok(z3::ast::Dynamic::from(Real::from_real(self.ctx, num as i32, den as i32)))
            }
            Expression::LiteralString(v) => {
                let z3_str = Z3String::from_str(self.ctx, v).map_err(|e| e.to_string())?;
                Ok(z3::ast::Dynamic::from(z3_str))
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
            Expression::MemberAccess { .. } => {
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
            Expression::Unary { op, expr } => {
                let val = self.eval_expr(expr, use_initial)?;
                if op == "!" {
                    let b = val.as_bool().ok_or("Negation expects a boolean expression")?;
                    Ok(z3::ast::Dynamic::from(b.not()))
                } else if op == "-" {
                    if let Some(i) = val.as_int() {
                        Ok(z3::ast::Dynamic::from(i.unary_minus()))
                    } else if let Some(r) = val.as_real() {
                        Ok(z3::ast::Dynamic::from(r.unary_minus()))
                    } else if let Some(bv) = val.as_bv() {
                        Ok(z3::ast::Dynamic::from(bv.bvneg()))
                    } else {
                        Err("Arithmetic negation expects an integer, real, or bitvector expression".to_string())
                    }
                } else {
                    Err(format!("Unsupported unary operator in SMT: {}", op))
                }
            }
            Expression::Call { function, args } => {
                if function == "initial" && args.len() == 1 {
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
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("Add expects integers")?;
                            Ok(z3::ast::Dynamic::from(Int::add(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("Add expects reals")?;
                            Ok(z3::ast::Dynamic::from(Real::add(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("Add expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(&l + &r))
                        } else {
                            Err("Unsupported types for Add".to_string())
                        }
                    }
                    BinaryOp::Sub => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("Sub expects integers")?;
                            Ok(z3::ast::Dynamic::from(Int::sub(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("Sub expects reals")?;
                            Ok(z3::ast::Dynamic::from(Real::sub(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("Sub expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(&l - &r))
                        } else {
                            Err("Unsupported types for Sub".to_string())
                        }
                    }
                    BinaryOp::Mul => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("Mul expects integers")?;
                            Ok(z3::ast::Dynamic::from(Int::mul(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("Mul expects reals")?;
                            Ok(z3::ast::Dynamic::from(Real::mul(self.ctx, &[&l, &r])))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("Mul expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(&l * &r))
                        } else {
                            Err("Unsupported types for Mul".to_string())
                        }
                    }
                    BinaryOp::Div => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("Div expects integers")?;
                            Ok(z3::ast::Dynamic::from(l.div(&r)))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("Div expects reals")?;
                            Ok(z3::ast::Dynamic::from(l.div(&r)))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("Div expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(l.bvsdiv(&r)))
                        } else {
                            Err("Unsupported types for Div".to_string())
                        }
                    }
                    BinaryOp::Eq => {
                        Ok(z3::ast::Dynamic::from(l_val._eq(&r_val)))
                    }
                    BinaryOp::Ne => {
                        Ok(z3::ast::Dynamic::from(l_val._eq(&r_val).not()))
                    }
                    BinaryOp::Lt => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("< expects integers")?;
                            Ok(z3::ast::Dynamic::from(l.lt(&r)))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("< expects reals")?;
                            Ok(z3::ast::Dynamic::from(l.lt(&r)))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("< expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(l.bvslt(&r)))
                        } else {
                            Err("Unsupported types for Lt".to_string())
                        }
                    }
                    BinaryOp::Le => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("<= expects integers")?;
                            Ok(z3::ast::Dynamic::from(l.le(&r)))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("<= expects reals")?;
                            Ok(z3::ast::Dynamic::from(l.le(&r)))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("<= expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(l.bvsle(&r)))
                        } else {
                            Err("Unsupported types for Le".to_string())
                        }
                    }
                    BinaryOp::Gt => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or("> expects integers")?;
                            Ok(z3::ast::Dynamic::from(l.gt(&r)))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or("> expects reals")?;
                            Ok(z3::ast::Dynamic::from(l.gt(&r)))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or("> expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(l.bvsgt(&r)))
                        } else {
                            Err("Unsupported types for Gt".to_string())
                        }
                    }
                    BinaryOp::Ge => {
                        if let Some(l) = l_val.as_int() {
                            let r = r_val.as_int().ok_or(">= expects integers")?;
                            Ok(z3::ast::Dynamic::from(l.ge(&r)))
                        } else if let Some(l) = l_val.as_real() {
                            let r = r_val.as_real().ok_or(">= expects reals")?;
                            Ok(z3::ast::Dynamic::from(l.ge(&r)))
                        } else if let Some(l) = l_val.as_bv() {
                            let r = r_val.as_bv().ok_or(">= expects bitvectors")?;
                            Ok(z3::ast::Dynamic::from(l.bvsge(&r)))
                        } else {
                            Err("Unsupported types for Ge".to_string())
                        }
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
            self.init_parameter(&solver, param)?;
        }

        // 2. Assert preconditions
        for prec in &intent.preconditions {
            let expr_z3 = self.eval_expr(prec, false)?;
            let b = expr_z3.as_bool().ok_or("Preconditions must be boolean expressions")?;
            solver.assert(&b);
        }

        // 3. Initial boundary check for invariants
        for inv in &intent.invariants {
            let expr_z3 = self.eval_expr(inv, false)?;
            let b = expr_z3.as_bool().ok_or("Invariants must be boolean expressions")?;
            solver.assert(&b);
        }

        // 4. Process function body statements (SSA generation)
        for stmt in &func.body {
            match stmt {
                Statement::Let { name, value, ty, .. } => {
                    let val_z3 = self.eval_expr(value, false)?;
                    let var_name = format!("{}_{}", name, self.variables.get(name).map(|v| v.len()).unwrap_or(0));
                    let new_var = if let Some(concrete_ty) = ty {
                        self.create_ssa_var(concrete_ty, &var_name)?
                    } else {
                        self.create_dynamic_ssa_var(&val_z3, &var_name)?
                    };

                    solver.assert(&new_var._eq(&val_z3));
                    self.push_var(name.clone(), new_var.clone());

                    // Verify refinement type obligation for the newly initialized variable!
                    if let Some(concrete_ty) = ty {
                        // We must verify that the value satisfies the refinement type.
                        // We do this by checking: if we assert the negation of refinement conditions, is it UNSAT?
                        // Let's create a temporary solver check or add it to the final assertions.
                        // Actually, check it immediately using a solver push/pop to verify!
                        solver.push();
                        // Get type refinement conditions for this variable:
                        // We can't use self.assert_type_refinement directly since it does solver.assert.
                        // Let's implement a helper `collect_type_refinements` that collects the booleans.
                        let mut ref_verifier = Verifier {
                            ctx: self.ctx,
                            variables: self.variables.clone(),
                            structs: self.structs.clone(),
                            type_aliases: self.type_aliases.clone(),
                        };
                        let refs = ref_verifier.collect_type_refinement_exprs(name, concrete_ty)?;
                        if !refs.is_empty() {
                            let all_refs = Bool::and(self.ctx, &refs.iter().collect::<Vec<_>>());
                            solver.assert(&all_refs.not());
                            if solver.check() == z3::SatResult::Sat {
                                return Err(format!("Verification Failed: Refinement constraint violated on let binding '{}'", name));
                            }
                        }
                        solver.pop(1);

                        // Now assert the refinement as a fact for subsequent statements
                        self.assert_type_refinement(&solver, name, concrete_ty)?;
                    }

                    // Check mid-block invariants after mutating states
                    for inv in &intent.invariants {
                        solver.push();
                        let expr_z3 = self.eval_expr(inv, false)?;
                        let b = expr_z3.as_bool().ok_or("Invariants must be boolean expressions")?;
                        solver.assert(&b.not());
                        if solver.check() == z3::SatResult::Sat {
                            return Err(format!("Verification Failed: Invariant violated mid-block on let binding '{}'", name));
                        }
                        solver.pop(1);
                        solver.assert(&b);
                    }
                }
                Statement::Assignment { lhs, op, rhs } => {
                    let path = self.resolve_path(lhs)?;
                    let rhs_z3 = self.eval_expr(rhs, false)?;
                    let current_lhs = self.get_current_var(&path).cloned()
                        .ok_or_else(|| format!("Assigning to undefined variable/path: {}", path))?;

                    let val_to_assign = match op {
                        Some(BinaryOp::Add) => {
                            if let Some(l) = current_lhs.as_int() {
                                let r = rhs_z3.as_int().ok_or("+= expects integer rhs")?;
                                z3::ast::Dynamic::from(Int::add(self.ctx, &[&l, &r]))
                            } else if let Some(l) = current_lhs.as_real() {
                                let r = rhs_z3.as_real().ok_or("+= expects real rhs")?;
                                z3::ast::Dynamic::from(Real::add(self.ctx, &[&l, &r]))
                            } else if let Some(l) = current_lhs.as_bv() {
                                let r = rhs_z3.as_bv().ok_or("+= expects bitvector rhs")?;
                                z3::ast::Dynamic::from(&l + &r)
                            } else {
                                return Err("Unsupported types for +=".to_string());
                            }
                        }
                        Some(BinaryOp::Sub) => {
                            if let Some(l) = current_lhs.as_int() {
                                let r = rhs_z3.as_int().ok_or("-= expects integer rhs")?;
                                z3::ast::Dynamic::from(Int::sub(self.ctx, &[&l, &r]))
                            } else if let Some(l) = current_lhs.as_real() {
                                let r = rhs_z3.as_real().ok_or("-= expects real rhs")?;
                                z3::ast::Dynamic::from(Real::sub(self.ctx, &[&l, &r]))
                            } else if let Some(l) = current_lhs.as_bv() {
                                let r = rhs_z3.as_bv().ok_or("-= expects bitvector rhs")?;
                                z3::ast::Dynamic::from(&l - &r)
                            } else {
                                return Err("Unsupported types for -=".to_string());
                            }
                        }
                        None => rhs_z3,
                        _ => return Err("Unsupported assignment operator in verifier".to_string()),
                    };

                    let new_var_name = format!("{}_{}", path.replace(".", "_"), self.variables.get(&path).map(|v| v.len()).unwrap_or(0));
                    let new_var = self.create_dynamic_ssa_var(&current_lhs, &new_var_name)?;
                    solver.assert(&new_var._eq(&val_to_assign));
                    self.push_var(path.clone(), new_var);

                    // Verify refinement type obligation for the newly assigned path
                    if let Some(ty) = self.get_path_type(&path, &func.parameters) {
                        solver.push();
                        let mut ref_verifier = Verifier {
                            ctx: self.ctx,
                            variables: self.variables.clone(),
                            structs: self.structs.clone(),
                            type_aliases: self.type_aliases.clone(),
                        };
                        let refs = ref_verifier.collect_type_refinement_exprs(&path, &ty)?;
                        if !refs.is_empty() {
                            let all_refs = Bool::and(self.ctx, &refs.iter().collect::<Vec<_>>());
                            solver.assert(&all_refs.not());
                            if solver.check() == z3::SatResult::Sat {
                                return Err(format!("Verification Failed: Refinement constraint violated on assignment to '{}'", path));
                            }
                        }
                        solver.pop(1);

                        // Assert refinement types as fact
                        self.assert_type_refinement(&solver, &path, &ty)?;
                    }

                    // Check mid-block invariants after mutating states
                    for inv in &intent.invariants {
                        solver.push();
                        let expr_z3 = self.eval_expr(inv, false)?;
                        let b = expr_z3.as_bool().ok_or("Invariants must be boolean expressions")?;
                        solver.assert(&b.not());
                        if solver.check() == z3::SatResult::Sat {
                            return Err(format!("Verification Failed: Invariant violated mid-block on assignment to '{}'", path));
                        }
                        solver.pop(1);
                        solver.assert(&b);
                    }
                }
                Statement::Expression(expr) => {
                    self.eval_expr(expr, false)?;
                }
            }
        }

        // 5. Evaluate postconditions and invariants terminal boundary checks
        // Negate both to search for bug counter-examples.
        let mut target_assertions = Vec::new();
        for post in &intent.postconditions {
            let expr_z3 = self.eval_expr(post, false)?;
            let b = expr_z3.as_bool().ok_or("Postconditions must be boolean expressions")?;
            target_assertions.push(b);
        }
        for inv in &intent.invariants {
            let expr_z3 = self.eval_expr(inv, false)?;
            let b = expr_z3.as_bool().ok_or("Invariants must be boolean expressions")?;
            target_assertions.push(b);
        }

        if !target_assertions.is_empty() {
            let all_targets = Bool::and(self.ctx, &target_assertions.iter().collect::<Vec<_>>());
            let negated_targets = all_targets.not();
            solver.assert(&negated_targets);
        }

        // 6. Run SMT Solver check
        match solver.check() {
            z3::SatResult::Unsat => Ok(()),
            z3::SatResult::Sat => {
                let model = solver.get_model().unwrap();
                let mut counter_example = String::new();
                counter_example.push_str("Verification Failed! Counter-example found:\n");
                
                // Print out the initial parameter values
                for param in &func.parameters {
                    self.print_counter_example_param(&mut counter_example, &model, &param.name, &param.ty);
                }
                
                Err(counter_example)
            }
            z3::SatResult::Unknown => {
                Err("Verification Unknown: Z3 could not determine satisfiability".to_string())
            }
        }
    }

    fn print_counter_example_param(&self, out: &mut String, model: &z3::Model<'ctx>, path: &str, ty: &Type) {
        match ty {
            Type::Int | Type::Bool | Type::Byte | Type::Float | Type::String => {
                if let Some(var) = self.get_initial_var(path) {
                    if let Some(val) = model.eval(var, true) {
                        out.push_str(&format!("  {} = {}\n", path, val));
                    }
                }
            }
            Type::Refinement { base, .. } => {
                self.print_counter_example_param(out, model, path, base);
            }
            Type::Custom(struct_name) => {
                if let Some(s_decl) = self.structs.get(struct_name) {
                    out.push_str(&format!("  {} (struct {}):\n", path, struct_name));
                    for field in &s_decl.fields {
                        let field_path = format!("{}.{}", path, field.name);
                        self.print_counter_example_param(out, model, &field_path, &field.ty);
                    }
                }
            }
            _ => {}
        }
    }

    fn collect_type_refinement_exprs(&mut self, path: &str, ty: &Type) -> Result<Vec<Bool<'ctx>>, String> {
        let mut refs = Vec::new();
        match ty {
            Type::Refinement { base, value_name, predicate } => {
                let var_z3 = self.get_current_var(path).cloned()
                    .ok_or_else(|| format!("Variable '{}' not found for refinement check", path))?;
                // Temporarily bind the variable value to value_name in scope
                self.push_var(value_name.to_string(), var_z3);
                let pred_z3 = self.eval_expr(predicate, false)?;
                let b = pred_z3.as_bool().ok_or("Refinement predicate must be a boolean expression")?;
                refs.push(b);
                if let Some(hist) = self.variables.get_mut(value_name) {
                    hist.pop();
                }
                refs.extend(self.collect_type_refinement_exprs(path, base)?);
            }
            Type::Custom(name) => {
                if let Some(aliased_ty) = self.type_aliases.get(name).cloned() {
                    refs.extend(self.collect_type_refinement_exprs(path, &aliased_ty)?);
                } else if let Some(s_decl) = self.structs.get(name).cloned() {
                    for field in &s_decl.fields {
                        let field_path = format!("{}.{}", path, field.name);
                        refs.extend(self.collect_type_refinement_exprs(&field_path, &field.ty)?);
                    }
                }
            }
            _ => {}
        }
        Ok(refs)
    }
}
