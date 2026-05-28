#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    String,
    Char,
    Byte,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Implies, // `->`
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    LiteralInt(i64),
    LiteralFloat(f64),
    LiteralBool(bool),
    LiteralString(String),
    Identifier(String),
    MemberAccess {
        object: Box<Expression>,
        member: String,
    },
    Binary {
        lhs: Box<Expression>,
        op: BinaryOp,
        rhs: Box<Expression>,
    },
    Unary {
        op: String,
        expr: Box<Expression>,
    },
    Call {
        function: String,
        args: Vec<Expression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        is_mut: bool,
        ty: Option<Type>,
        value: Expression,
    },
    Assignment {
        lhs: Expression,
        op: Option<BinaryOp>, // Some(BinaryOp::Sub) for `-=`
        rhs: Expression,
    },
    Expression(Expression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub is_mut: bool,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDeclaration {
    pub name: String,
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntentDeclaration {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub preconditions: Vec<Expression>,
    pub postconditions: Vec<Expression>,
    pub invariants: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDeclaration {
    pub is_pub: bool,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub satisfies: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TopLevel {
    Module(String),
    Struct(StructDeclaration),
    Intent(IntentDeclaration),
    Function(FunctionDeclaration),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<TopLevel>,
}
