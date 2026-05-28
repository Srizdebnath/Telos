use crate::ast::*;

pub struct Codegen;

impl Codegen {
    pub fn transpile(program: &Program) -> String {
        let mut out = String::new();
        for decl in &program.declarations {
            match decl {
                TopLevel::Module(name) => {
                    out.push_str(&format!("// Module: {}\n\n", name));
                }
                TopLevel::Struct(s) => {
                    out.push_str("#[derive(Debug, Clone)]\n");
                    out.push_str(&format!("pub struct {} {{\n", s.name));
                    for field in &s.fields {
                        out.push_str(&format!("    pub {}: {},\n", field.name, Self::transpile_type(&field.ty)));
                    }
                    out.push_str("}\n\n");
                }
                TopLevel::Intent(intent) => {
                    // Intents are verification-only contracts, they are commented out in the transpiled output
                    out.push_str(&format!("/*\nintent {}(...) {{\n", intent.name));
                    out.push_str("    preconditions: ...\n");
                    out.push_str("    postconditions: ...\n");
                    out.push_str("}\n*/\n\n");
                }
                TopLevel::Function(func) => {
                    let vis = if func.is_pub { "pub " } else { "" };
                    let params: Vec<String> = func.parameters.iter().map(|p| {
                        let mut_flag = if p.is_mut { "mut " } else { "" };
                        format!("{}{}: {}", mut_flag, p.name, Self::transpile_type(&p.ty))
                    }).collect();

                    let ret = match &func.return_type {
                        Some(ty) => format!(" -> {}", Self::transpile_type(ty)),
                        None => String::new(),
                    };

                    out.push_str(&format!("{}fn {}({}){} {{\n", vis, func.name, params.join(", "), ret));
                    for stmt in &func.body {
                        out.push_str(&format!("    {}\n", Self::transpile_statement(stmt)));
                    }
                    out.push_str("}\n\n");
                }
            }
        }
        out
    }

    fn transpile_type(ty: &Type) -> String {
        match ty {
            Type::Int => "i64".to_string(),
            Type::Float => "f64".to_string(),
            Type::Bool => "bool".to_string(),
            Type::String => "String".to_string(),
            Type::Char => "char".to_string(),
            Type::Byte => "u8".to_string(),
            Type::Custom(name) => name.clone(),
        }
    }

    fn transpile_statement(stmt: &Statement) -> String {
        match stmt {
            Statement::Let { name, is_mut, ty, value } => {
                let mut_flag = if *is_mut { "mut " } else { "" };
                let ty_str = match ty {
                    Some(t) => format!(": {}", Self::transpile_type(t)),
                    None => String::new(),
                };
                format!("let {}{}{} = {};", mut_flag, name, ty_str, Self::transpile_expression(value))
            }
            Statement::Assignment { lhs, op, rhs } => {
                let op_str = match op {
                    Some(BinaryOp::Add) => "+=",
                    Some(BinaryOp::Sub) => "-=",
                    Some(BinaryOp::Mul) => "*=",
                    Some(BinaryOp::Div) => "/=",
                    None => "=",
                    _ => panic!("Unsupported assignment operator"),
                };
                format!("{} {} {};", Self::transpile_expression(lhs), op_str, Self::transpile_expression(rhs))
            }
            Statement::Expression(expr) => {
                format!("{};", Self::transpile_expression(expr))
            }
        }
    }

    fn transpile_expression(expr: &Expression) -> String {
        match expr {
            Expression::LiteralInt(v) => v.to_string(),
            Expression::LiteralFloat(v) => v.to_string(),
            Expression::LiteralBool(v) => v.to_string(),
            Expression::LiteralString(v) => format!("\"{}\".to_string()", v),
            Expression::Identifier(name) => name.clone(),
            Expression::MemberAccess { object, member } => {
                format!("{}.{}", Self::transpile_expression(object), member)
            }
            Expression::Binary { lhs, op, rhs } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    BinaryOp::Implies => "==", // Logical implication can be mapped to == or handled (a -> b is !a || b)
                };
                if let BinaryOp::Implies = op {
                    format!("(!{} || {})", Self::transpile_expression(lhs), Self::transpile_expression(rhs))
                } else {
                    format!("({} {} {})", Self::transpile_expression(lhs), op_str, Self::transpile_expression(rhs))
                }
            }
            Expression::Unary { op, expr } => {
                format!("({}{})", op, Self::transpile_expression(expr))
            }
            Expression::Call { function, args } => {
                let args_str: Vec<String> = args.iter().map(Self::transpile_expression).collect();
                format!("{}({})", function, args_str.join(", "))
            }
        }
    }
}
