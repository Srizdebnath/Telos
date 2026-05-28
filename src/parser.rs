use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
    tokens: Vec<(Token, usize, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Token, usize, usize)>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(t, _, _)| t)
    }

    fn peek_pos(&self) -> (usize, usize) {
        self.tokens.get(self.pos)
            .map(|(_, l, c)| (*l, *c))
            .unwrap_or((0, 0))
    }

    fn advance(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let (t, _, _) = &self.tokens[self.pos];
            self.pos += 1;
            Some(t.clone())
        } else {
            None
        }
    }

    fn check(&self, tok: &Token) -> bool {
        self.peek() == Some(tok)
    }

    fn match_token(&mut self, tok: &Token) -> bool {
        if self.check(tok) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, tok: &Token) -> Result<Token, String> {
        if self.check(tok) {
            Ok(self.advance().unwrap())
        } else {
            let (l, c) = self.peek_pos();
            Err(format!("{}:{}: Expected token {:?}, found {:?}", l, c, tok, self.peek()))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, String> {
        let (l, c) = self.peek_pos();
        match self.peek() {
            Some(Token::Identifier(_)) => {
                if let Some(Token::Identifier(name)) = self.advance() {
                    Ok(name)
                } else {
                    unreachable!()
                }
            }
            other => Err(format!("{}:{}: Expected identifier, found {:?}", l, c, other)),
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, String> {
        let mut declarations = Vec::new();
        while self.pos < self.tokens.len() {
            if self.match_token(&Token::Module) {
                let name = self.expect_identifier()?;
                declarations.push(TopLevel::Module(name));
            } else if self.check(&Token::TypeKeyword) {
                let mut is_struct = false;
                if self.pos + 3 < self.tokens.len() {
                    if let (Token::Eq, Token::Struct) = (&self.tokens[self.pos + 2].0, &self.tokens[self.pos + 3].0) {
                        is_struct = true;
                    }
                }
                if is_struct {
                    declarations.push(TopLevel::Struct(self.parse_struct()?));
                } else {
                    declarations.push(TopLevel::TypeAlias(self.parse_type_alias()?));
                }
            } else if self.check(&Token::Intent) {
                declarations.push(TopLevel::Intent(self.parse_intent()?));
            } else if self.check(&Token::Fn) || self.check(&Token::Pub) {
                declarations.push(TopLevel::Function(self.parse_function()?));
            } else {
                let (l, c) = self.peek_pos();
                return Err(format!("{}:{}: Unexpected token in top-level: {:?}", l, c, self.peek()));
            }
        }
        Ok(Program { declarations })
    }

    fn parse_type_alias(&mut self) -> Result<TypeAliasDeclaration, String> {
        self.expect(&Token::TypeKeyword)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Eq)?;
        let ty = self.parse_type()?;
        self.match_token(&Token::Semicolon);
        Ok(TypeAliasDeclaration { name, ty })
    }

    fn parse_struct(&mut self) -> Result<StructDeclaration, String> {
        self.expect(&Token::TypeKeyword)?;
        let name = self.expect_identifier()?;
        self.expect(&Token::Eq)?;
        self.expect(&Token::Struct)?;
        self.expect(&Token::LBrace)?;
        
        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) {
            let field_name = self.expect_identifier()?;
            self.expect(&Token::Colon)?;
            let field_type = self.parse_type()?;
            fields.push(StructField { name: field_name, ty: field_type });
            if !self.match_token(&Token::Comma) {
                if self.check(&Token::RBrace) {
                    break;
                }
                let (l, c) = self.peek_pos();
                return Err(format!("{}:{}: Expected ',' or '}}' after struct field, found {:?}", l, c, self.peek()));
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(StructDeclaration { name, fields })
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        let (l, c) = self.peek_pos();
        let base = match self.advance() {
            Some(Token::Identifier(ref s)) => match s.as_str() {
                "Int" => Type::Int,
                "Float" => Type::Float,
                "Bool" => Type::Bool,
                "String" => Type::String,
                "Char" => Type::Char,
                "Byte" => Type::Byte,
                other => Type::Custom(other.to_string()),
            },
            other => return Err(format!("{}:{}: Expected type name, found {:?}", l, c, other)),
        };

        if self.match_token(&Token::Where) {
            let value_name = match self.peek() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => "value".to_string(),
            };
            let predicate = self.parse_expression()?;
            Ok(Type::Refinement {
                base: Box::new(base),
                value_name,
                predicate,
            })
        } else {
            Ok(base)
        }
    }

    fn parse_parameter_list(&mut self) -> Result<Vec<Parameter>, String> {
        self.expect(&Token::LParen)?;
        let mut params = Vec::new();
        while !self.check(&Token::RParen) {
            let is_mut = self.match_token(&Token::Mut);
            let name = self.expect_identifier()?;
            self.expect(&Token::Colon)?;
            let ty = self.parse_type()?;
            params.push(Parameter { is_mut, name, ty });
            if !self.match_token(&Token::Comma) {
                if self.check(&Token::RParen) {
                    break;
                }
                let (l, c) = self.peek_pos();
                return Err(format!("{}:{}: Expected ',' or ')' in parameters, found {:?}", l, c, self.peek()));
            }
        }
        self.expect(&Token::RParen)?;
        Ok(params)
    }

    fn parse_intent(&mut self) -> Result<IntentDeclaration, String> {
        self.expect(&Token::Intent)?;
        let name = self.expect_identifier()?;
        let parameters = self.parse_parameter_list()?;
        self.expect(&Token::LBrace)?;

        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();
        let mut invariants = Vec::new();

        while !self.check(&Token::RBrace) {
            let (l, c) = self.peek_pos();
            match self.advance() {
                Some(Token::Preconditions) => {
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    preconditions = self.parse_expression_list()?;
                    self.expect(&Token::RBracket)?;
                }
                Some(Token::Postconditions) => {
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    postconditions = self.parse_expression_list()?;
                    self.expect(&Token::RBracket)?;
                }
                Some(Token::Invariants) => {
                    self.expect(&Token::Colon)?;
                    self.expect(&Token::LBracket)?;
                    invariants = self.parse_expression_list()?;
                    self.expect(&Token::RBracket)?;
                }
                other => return Err(format!("{}:{}: Expected intent section name (preconditions, postconditions, invariants), found {:?}", l, c, other)),
            }
            self.match_token(&Token::Comma); // optional comma between sections
        }
        self.expect(&Token::RBrace)?;
        Ok(IntentDeclaration {
            name,
            parameters,
            preconditions,
            postconditions,
            invariants,
        })
    }

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, String> {
        let mut exprs = Vec::new();
        while !self.check(&Token::RBracket) {
            exprs.push(self.parse_expression()?);
            if !self.match_token(&Token::Comma) {
                if self.check(&Token::RBracket) {
                    break;
                }
                let (l, c) = self.peek_pos();
                return Err(format!("{}:{}: Expected ',' or ']' in expression list, found {:?}", l, c, self.peek()));
            }
        }
        Ok(exprs)
    }

    fn parse_function(&mut self) -> Result<FunctionDeclaration, String> {
        let is_pub = self.match_token(&Token::Pub);
        self.expect(&Token::Fn)?;
        let name = self.expect_identifier()?;
        let parameters = self.parse_parameter_list()?;
        
        let mut return_type = None;
        if self.match_token(&Token::Arrow) {
            return_type = Some(self.parse_type()?);
        }

        let mut satisfies = None;
        if self.match_token(&Token::Satisfies) {
            satisfies = Some(self.expect_identifier()?);
        }

        self.expect(&Token::LBrace)?;
        let mut body = Vec::new();
        while !self.check(&Token::RBrace) {
            body.push(self.parse_statement()?);
        }
        self.expect(&Token::RBrace)?;

        Ok(FunctionDeclaration {
            is_pub,
            name,
            parameters,
            return_type,
            satisfies,
            body,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(&Token::Let) {
            let is_mut = self.match_token(&Token::Mut);
            let name = self.expect_identifier()?;
            let mut ty = None;
            if self.match_token(&Token::Colon) {
                ty = Some(self.parse_type()?);
            }
            self.expect(&Token::Eq)?;
            let value = self.parse_expression()?;
            self.expect(&Token::Semicolon)?;
            Ok(Statement::Let { name, is_mut, ty, value })
        } else {
            // Can be assignment or simple expression statement
            let expr = self.parse_expression()?;
            if self.match_token(&Token::Eq) {
                let rhs = self.parse_expression()?;
                self.expect(&Token::Semicolon)?;
                Ok(Statement::Assignment { lhs: expr, op: None, rhs })
            } else if self.match_token(&Token::PlusEq) {
                let rhs = self.parse_expression()?;
                self.expect(&Token::Semicolon)?;
                Ok(Statement::Assignment { lhs: expr, op: Some(BinaryOp::Add), rhs })
            } else if self.match_token(&Token::MinusEq) {
                let rhs = self.parse_expression()?;
                self.expect(&Token::Semicolon)?;
                Ok(Statement::Assignment { lhs: expr, op: Some(BinaryOp::Sub), rhs })
            } else {
                self.match_token(&Token::Semicolon); // optional semicolon for expression statements
                Ok(Statement::Expression(expr))
            }
        }
    }

    pub fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_implies()
    }

    fn parse_implies(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_logical_or()?;
        while self.match_token(&Token::Arrow) {
            let rhs = self.parse_logical_or()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op: BinaryOp::Implies,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_logical_or(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_logical_and()?;
        while self.match_token(&Token::PipePipe) {
            let rhs = self.parse_logical_and()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op: BinaryOp::Or,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_logical_and(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_equality()?;
        while self.match_token(&Token::AmpAmp) {
            let rhs = self.parse_equality()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op: BinaryOp::And,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_equality(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_comparison()?;
        while self.check(&Token::EqEq) || self.check(&Token::Ne) {
            let op = if self.match_token(&Token::EqEq) {
                BinaryOp::Eq
            } else {
                self.advance();
                BinaryOp::Ne
            };
            let rhs = self.parse_comparison()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_term()?;
        while self.check(&Token::Lt) || self.check(&Token::Le) || self.check(&Token::Gt) || self.check(&Token::Ge) {
            let op = if self.match_token(&Token::Lt) {
                BinaryOp::Lt
            } else if self.match_token(&Token::Le) {
                BinaryOp::Le
            } else if self.match_token(&Token::Gt) {
                BinaryOp::Gt
            } else {
                self.advance();
                BinaryOp::Ge
            };
            let rhs = self.parse_term()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_term(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_factor()?;
        while self.check(&Token::Plus) || self.check(&Token::Minus) {
            let op = if self.match_token(&Token::Plus) {
                BinaryOp::Add
            } else {
                self.advance();
                BinaryOp::Sub
            };
            let rhs = self.parse_factor()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_factor(&mut self) -> Result<Expression, String> {
        let mut lhs = self.parse_primary()?;
        while self.check(&Token::Asterisk) || self.check(&Token::Slash) {
            let op = if self.match_token(&Token::Asterisk) {
                BinaryOp::Mul
            } else {
                self.advance();
                BinaryOp::Div
            };
            let rhs = self.parse_primary()?;
            lhs = Expression::Binary {
                lhs: Box::new(lhs),
                op,
                rhs: Box::new(rhs),
            };
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let (l, c) = self.peek_pos();
        let mut expr = match self.advance() {
            Some(Token::IntLiteral(v)) => Expression::LiteralInt(v),
            Some(Token::FloatLiteral(v)) => Expression::LiteralFloat(v),
            Some(Token::BoolLiteral(v)) => Expression::LiteralBool(v),
            Some(Token::StringLiteral(v)) => Expression::LiteralString(v),
            Some(Token::Identifier(name)) => {
                // Check if it's a function call
                if self.match_token(&Token::LParen) {
                    let mut args = Vec::new();
                    while !self.check(&Token::RParen) {
                        args.push(self.parse_expression()?);
                        if !self.match_token(&Token::Comma) {
                            if self.check(&Token::RParen) {
                                break;
                            }
                            let (l, c) = self.peek_pos();
                            return Err(format!("{}:{}: Expected ',' or ')' in args list, found {:?}", l, c, self.peek()));
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Expression::Call { function: name, args }
                } else {
                    Expression::Identifier(name)
                }
            }
            Some(Token::LParen) => {
                let inner = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                inner
            }
            Some(Token::Bang) => {
                Expression::Unary {
                    op: "!".to_string(),
                    expr: Box::new(self.parse_primary()?),
                }
            }
            Some(Token::Minus) => {
                Expression::Unary {
                    op: "-".to_string(),
                    expr: Box::new(self.parse_primary()?),
                }
            }
            Some(Token::Amp) => {
                let op = if self.match_token(&Token::Mut) {
                    "&mut".to_string()
                } else {
                    "&".to_string()
                };
                Expression::Unary {
                    op,
                    expr: Box::new(self.parse_primary()?),
                }
            }
            other => return Err(format!("{}:{}: Expected primary expression, found {:?}", l, c, other)),
        };

        // Handle trailing member access (e.g. source.balance)
        while self.match_token(&Token::Dot) {
            let member = self.expect_identifier()?;
            expr = Expression::MemberAccess {
                object: Box::new(expr),
                member,
            };
        }

        Ok(expr)
    }
}
