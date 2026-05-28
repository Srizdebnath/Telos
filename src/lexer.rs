#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Module,
    Struct,
    Intent,
    Preconditions,
    Postconditions,
    Invariants,
    Fn,
    Pub,
    Satisfies,
    Let,
    Mut,
    Where,
    TypeKeyword,
    
    // Literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),

    // Operators & Punctuation
    Plus,
    Minus,
    Asterisk,
    Slash,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    PlusEq,
    MinusEq,
    Arrow, // ->
    Amp, // &
    AmpAmp, // &&
    PipePipe, // ||
    Bang, // !
    Comma,
    Colon,
    Dot,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
}

pub struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
            line: 1,
            col: 1,
        }
    }

    fn next_char(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            if ch == '\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
        c
    }

    fn peek_char(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    pub fn tokenize(&mut self) -> Result<Vec<(Token, usize, usize)>, String> {
        let mut tokens = Vec::new();
        while let Some(&c) = self.peek_char() {
            let start_line = self.line;
            let start_col = self.col;
            if c.is_whitespace() {
                self.next_char();
                continue;
            }

            // Comments
            if c == '/' {
                self.next_char();
                if self.peek_char() == Some(&'/') {
                    // Line comment
                    while let Some(ch) = self.next_char() {
                        if ch == '\n' {
                            break;
                        }
                    }
                    continue;
                } else {
                    tokens.push((Token::Slash, start_line, start_col));
                    continue;
                }
            }

            if c.is_ascii_alphabetic() || c == '_' {
                let mut ident = String::new();
                while let Some(&ch) = self.peek_char() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ident.push(self.next_char().unwrap());
                    } else {
                        break;
                    }
                }

                let token = match ident.as_str() {
                    "module" => Token::Module,
                    "struct" => Token::Struct,
                    "intent" => Token::Intent,
                    "preconditions" => Token::Preconditions,
                    "postconditions" => Token::Postconditions,
                    "invariants" => Token::Invariants,
                    "fn" => Token::Fn,
                    "pub" => Token::Pub,
                    "satisfies" => Token::Satisfies,
                    "let" => Token::Let,
                    "mut" => Token::Mut,
                    "where" => Token::Where,
                    "type" => Token::TypeKeyword,
                    "true" => Token::BoolLiteral(true),
                    "false" => Token::BoolLiteral(false),
                    _ => Token::Identifier(ident),
                };
                tokens.push((token, start_line, start_col));
                continue;
            }

            if c.is_ascii_digit() {
                let mut num = String::new();
                let mut is_float = false;
                while let Some(&ch) = self.peek_char() {
                    if ch.is_ascii_digit() {
                        num.push(self.next_char().unwrap());
                    } else if ch == '.' {
                        // Check if it's float or range/member access. If followed by a digit, it's a float.
                        // Peek further or just check next char.
                        self.next_char();
                        if let Some(&next_ch) = self.peek_char() {
                            if next_ch.is_ascii_digit() {
                                num.push('.');
                                num.push(self.next_char().unwrap());
                                is_float = true;
                            } else {
                                // Just a dot following an integer. Push integer token, then dot.
                                // But since we already consumed the dot:
                                let val: i64 = num.parse().map_err(|e| format!("{}:{}: Invalid int: {}", start_line, start_col, e))?;
                                tokens.push((Token::IntLiteral(val), start_line, start_col));
                                tokens.push((Token::Dot, self.line, self.col - 1));
                                num.clear();
                                break;
                            }
                        } else {
                            let val: i64 = num.parse().map_err(|e| format!("{}:{}: Invalid int: {}", start_line, start_col, e))?;
                            tokens.push((Token::IntLiteral(val), start_line, start_col));
                            tokens.push((Token::Dot, self.line, self.col - 1));
                            num.clear();
                            break;
                        }
                    } else {
                        break;
                    }
                }

                if !num.is_empty() {
                    if is_float {
                        let val: f64 = num.parse().map_err(|e| format!("{}:{}: Invalid float: {}", start_line, start_col, e))?;
                        tokens.push((Token::FloatLiteral(val), start_line, start_col));
                    } else {
                        let val: i64 = num.parse().map_err(|e| format!("{}:{}: Invalid int: {}", start_line, start_col, e))?;
                        tokens.push((Token::IntLiteral(val), start_line, start_col));
                    }
                }
                continue;
            }

            if c == '"' {
                self.next_char();
                let mut s = String::new();
                let mut closed = false;
                while let Some(ch) = self.next_char() {
                    if ch == '"' {
                        closed = true;
                        break;
                    }
                    s.push(ch);
                }
                if !closed {
                    return Err(format!("{}:{}: Unterminated string literal", start_line, start_col));
                }
                tokens.push((Token::StringLiteral(s), start_line, start_col));
                continue;
            }

            // Single and multi-character symbols
            self.next_char();
            let tok = match c {
                '+' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::PlusEq
                    } else {
                        Token::Plus
                    }
                }
                '-' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::MinusEq
                    } else if self.peek_char() == Some(&'>') {
                        self.next_char();
                        Token::Arrow
                    } else {
                        Token::Minus
                    }
                }
                '*' => Token::Asterisk,
                '=' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::EqEq
                    } else {
                        Token::Eq
                    }
                }
                '!' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::Ne
                    } else {
                        Token::Bang
                    }
                }
                '<' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::Le
                    } else {
                        Token::Lt
                    }
                }
                '>' => {
                    if self.peek_char() == Some(&'=') {
                        self.next_char();
                        Token::Ge
                    } else {
                        Token::Gt
                    }
                }
                '&' => {
                    if self.peek_char() == Some(&'&') {
                        self.next_char();
                        Token::AmpAmp
                    } else {
                        Token::Amp
                    }
                }
                '|' => {
                    if self.peek_char() == Some(&'|') {
                        self.next_char();
                        Token::PipePipe
                    } else {
                        return Err(format!("{}:{}: Single '|' is not supported", start_line, start_col));
                    }
                }
                ',' => Token::Comma,
                ':' => Token::Colon,
                '.' => Token::Dot,
                '(' => Token::LParen,
                ')' => Token::RParen,
                '{' => Token::LBrace,
                '}' => Token::RBrace,
                '[' => Token::LBracket,
                ']' => Token::RBracket,
                ';' => Token::Semicolon,
                _ => return Err(format!("{}:{}: Unexpected character '{}'", start_line, start_col, c)),
            };
            tokens.push((tok, start_line, start_col));
        }
        Ok(tokens)
    }
}
