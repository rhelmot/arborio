use celeste::binel::BinElAttr;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::character::complete::{one_of, space0};
use nom::combinator::{complete, eof, map, map_res, opt, recognize};
use nom::error::{Error, ErrorKind};
use nom::multi::{fold_many0, many0, separated_list0};
use nom::number::complete;
use nom::sequence::{delimited, pair, separated_pair, terminated, tuple};
use nom::{IResult, InputTakeAtPosition, Parser};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq)] // TODO WHY DO WE NEED CLONE OMG
pub enum Expression {
    Const(Const),
    Atom(String),
    BinOp(BinOp, Box<(Expression, Expression)>),
    UnOp(UnOp, Box<Expression>),
    Match {
        test: Box<Expression>,
        arms: HashMap<Const, Expression>,
        default: Box<Expression>,
    },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

impl BinOp {
    fn as_str(&self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "%",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
        }
    }
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Exists,
}

impl UnOp {
    fn as_str(&self) -> &'static str {
        match self {
            UnOp::Neg => "-",
            UnOp::Exists => "?",
        }
    }
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Const {
    Number(Number),
    String(String),
}

impl std::fmt::Display for Const {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Const::Number(n) => write!(f, "{}", n),
            Const::String(s) => {
                if !s.contains('\"') {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "\'{}\'", s)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Number(pub f64);

impl Number {
    pub fn to_int(&self) -> i32 {
        self.0 as i32
    }

    pub fn to_float(&self) -> f32 {
        self.0 as f32
    }
}

impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// aggressively terrible solution to the floating point comparison problem:
// force all nan == nan, use normal fp comparison for others
impl PartialEq for Number {
    fn eq(&self, other: &Number) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            "nan".hash(state);
        } else {
            self.0.to_bits().hash(state);
        }
    }
}

fn num_lit(input: &str) -> IResult<&str, Number> {
    map(alt((hex_num, complete::double)), Number)(input)
}

fn hex_num(input: &str) -> IResult<&str, f64> {
    map_res(
        complete(tuple((
            opt(alt((tag("-"), tag("+")))),
            alt((tag("0x"), tag("0X"))),
            nom::character::complete::hex_digit1,
        ))),
        |(sign, _, digits)| {
            i64::from_str_radix(digits, 16)
                .map(|q| (q as f64) * if sign == Some("-") { -1f64 } else { 1f64 })
        },
    )(input)
}

fn not_char(ch: char) -> impl FnMut(&str) -> IResult<&str, &str> {
    move |input: &str| input.split_at_position(|ch2| ch2 == ch)
}

// TODO use escaped_transform
fn string_lit_dquote(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), not_char('"'), tag("\""))(input)
}

fn string_lit_squote(input: &str) -> IResult<&str, &str> {
    delimited(tag("'"), not_char('\''), tag("'"))(input)
}

fn string_lit(input: &str) -> IResult<&str, &str> {
    alt((string_lit_dquote, string_lit_squote))(input)
}

const IDENT_START_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const IDENT_CONT_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn atom(input: &str) -> IResult<&str, Expression> {
    map(
        delimited(
            space0,
            recognize(pair(
                one_of(IDENT_START_CHARS),
                many0(complete(one_of(IDENT_CONT_CHARS))),
            )),
            space0,
        ),
        |s: &str| Expression::Atom(s.to_owned()),
    )(input)
}

fn string_const(input: &str) -> IResult<&str, Const> {
    delimited(space0, string_lit, space0)
        .map(str::to_owned)
        .map(Const::String)
        .parse(input)
}

fn num_const(input: &str) -> IResult<&str, Const> {
    map(delimited(space0, num_lit, space0), Const::Number)(input)
}

fn const_expression(input: &str) -> IResult<&str, Expression> {
    alt((string_const, num_const))
        .map(Expression::Const)
        .parse(input)
}

fn parenthetical(input: &str) -> IResult<&str, Expression> {
    delimited(
        tuple((space0, tag("("), space0)),
        expression_4,
        tuple((space0, tag(")"), space0)),
    )(input)
}

fn match_case_const(input: &str) -> IResult<&str, Option<Const>> {
    alt((string_const, num_const)).map(Some).parse(input)
}

fn match_case_default(input: &str) -> IResult<&str, Option<Const>> {
    delimited(space0, tag("_"), space0)
        .map(|_| None)
        .parse(input)
}

fn match_case(input: &str) -> IResult<&str, Option<Const>> {
    alt((match_case_const, match_case_default))(input)
}

fn match_arm(input: &str) -> IResult<&str, (Option<Const>, Expression)> {
    separated_pair(
        match_case,
        delimited(space0, tag("=>"), space0),
        expression_4,
    )(input)
}

fn construct_match_expr(
    test: Expression,
    arms_list: Vec<(Option<Const>, Expression)>,
) -> Result<Expression, ()> {
    let mut arms: HashMap<Const, Expression> = HashMap::new();
    let mut default: Option<Expression> = None;
    for (case, expr) in arms_list {
        match case {
            Some(real_case) => {
                if arms.contains_key(&real_case) {
                    return Err(());
                }
                arms.insert(real_case, expr);
            }
            None => {
                if default.is_some() {
                    return Err(());
                }
                default = Some(expr);
            }
        }
    }

    if let Some(default) = default {
        Ok(Expression::Match {
            test: Box::new(test),
            arms,
            default: Box::new(default),
        })
    } else {
        Err(())
    }
}

fn match_expr_with_errors(input: &str) -> IResult<&str, Result<Expression, ()>> {
    pair(
        delimited(
            tuple((space0, pair(tag("match"), space0))),
            expression_4,
            space0,
        ),
        delimited(
            pair(tag("{"), space0),
            separated_list0(delimited(space0, tag(","), space0), match_arm),
            pair(space0, tag("}")),
        ),
    )
    .map(|(test, arms_list)| construct_match_expr(test, arms_list))
    .parse(input)
}

fn match_expr(input: &str) -> IResult<&str, Expression> {
    map_res(match_expr_with_errors, |s| {
        s.map_err(|_| Error {
            input: "what",
            code: ErrorKind::MapRes,
        })
    })(input)
}

fn bin_op(op: BinOp) -> impl FnMut(&str) -> IResult<&str, BinOp> {
    move |input| {
        map(tag(op.as_str()), move |_| op)(input)
    }
}

fn un_op(op: UnOp) -> impl FnMut(&str) -> IResult<&str, UnOp> {
    move |input| map(tag(op.as_str()), move |_| op)(input)
}

fn bin_expression<'a>(
    mut operator: impl Parser<&'a str, BinOp, Error<&'a str>>,
    mut sub_expression: impl Parser<&'a str, Expression, Error<&'a str>>,
) -> impl FnMut(&'a str) -> IResult<&'a str, Expression> {
    move |input: &str| {
        let (input, init) = sub_expression.parse(input)?;
        let mut init = Some(init);
        fold_many0(
            complete(pair(
                delimited(space0, |input| operator.parse(input), space0),
                |input| sub_expression.parse(input),
            )),
            move || init.take().unwrap(),
            |acc, (op, rhs)| Expression::BinOp(op, Box::new((acc, rhs))),
        )(input)
    }
}

fn operator_4(input: &str) -> IResult<&str, BinOp> {
    use BinOp::*;
    alt((
        bin_op(Ge),
        bin_op(Le),
        bin_op(Gt),
        bin_op(Lt),
        bin_op(Eq),
        bin_op(Ne),
    ))(input)
}

fn expression_4(input: &str) -> IResult<&str, Expression> {
    bin_expression(operator_4, expression_3)(input)
}

fn operator_3(input: &str) -> IResult<&str, BinOp> {
    use BinOp::*;
    alt((bin_op(Add), bin_op(Sub)))(input)
}

fn expression_3(input: &str) -> IResult<&str, Expression> {
    bin_expression(operator_3, expression_2)(input)
}

fn operator_2(input: &str) -> IResult<&str, BinOp> {
    use BinOp::*;
    alt((bin_op(Mul), bin_op(Div), bin_op(Mod)))(input)
}

fn expression_2(input: &str) -> IResult<&str, Expression> {
    bin_expression(operator_2, expression_1)(input)
}

fn expression_1(input: &str) -> IResult<&str, Expression> {
    use UnOp::*;
    alt((
        expression_0,
        pair(
            delimited(space0, alt((un_op(Neg), un_op(Exists))), space0),
            expression_1,
        )
        .map(|(op, expr)| Expression::UnOp(op, Box::new(expr))),
    ))(input)
}

fn expression_0(input: &str) -> IResult<&str, Expression> {
    // match must go first in the list since it could be interpreted as an atom
    alt((match_expr, const_expression, atom, parenthetical))(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
    terminated(expression_4, eof)(input)
}

impl<'de> Deserialize<'de> for Expression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        let parsed = expression(s.as_str());
        if let Err(e) = parsed {
            dbg!(e);
            panic!("Error parsing {}", s); // TODO ummmm how do you construct this kind of error
        }
        Ok(parsed.unwrap().1)
    }
}

impl Serialize for Expression {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("{}", self).serialize(s)
    }
}

impl std::fmt::Display for Expression {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expression::Const(c) => write!(f, "{}", c),
            Expression::Atom(s) => write!(f, "{}", s),
            // TODO analyze operator precedence to tell if parens are necessary
            Expression::BinOp(op, children) => write!(
                f,
                "({} {} {})",
                children.as_ref().0,
                op,
                children.as_ref().1
            ),
            Expression::UnOp(op, child) => write!(f, "{}{}", op, child.as_ref()),
            Expression::Match {
                test,
                arms,
                default,
            } => {
                write!(f, "match {} {{ ", test.as_ref())?;
                for (matching, expression) in arms {
                    write!(f, "{} => {}, ", matching, expression)?;
                }
                write!(f, "_ => {} }}", default.as_ref())
            }
        }
    }
}

impl Expression {
    pub fn mk_const(con: i32) -> Expression {
        Expression::Const(Const::Number(Number(con as f64)))
    }

    pub fn evaluate(&self, env: &HashMap<&str, Const>) -> Result<Const, String> {
        match self {
            Expression::Const(c) => Ok(c.clone()),
            Expression::Atom(name) => env
                .get(name.as_str())
                .cloned()
                .ok_or_else(|| format!("Name \"{}\" undefined", name)),
            Expression::BinOp(op, children) => {
                let child1val = children.as_ref().0.evaluate(env)?;
                let child2val = children.as_ref().1.evaluate(env)?;
                match op {
                    BinOp::Add => {
                        if let (&Const::Number(Number(n1)), &Const::Number(Number(n2))) =
                            (&child1val, &child2val)
                        {
                            Ok(Const::Number(Number(n1 + n2)))
                        } else {
                            Ok(Const::String(
                                child1val.as_string()? + &child2val.as_string()?,
                            ))
                        }
                    }
                    BinOp::Sub => Ok(Const::Number(Number(
                        child1val.as_number()?.0 - child2val.as_number()?.0,
                    ))),
                    BinOp::Mul => Ok(Const::Number(Number(
                        child1val.as_number()?.0 * child2val.as_number()?.0,
                    ))),
                    // division by zero can produce nan and that's okay (?)
                    BinOp::Div => Ok(Const::Number(Number(
                        child1val.as_number()?.0 / child2val.as_number()?.0,
                    ))),
                    BinOp::Mod => Ok(Const::Number(Number(
                        child1val.as_number()?.0 % child2val.as_number()?.0,
                    ))),
                    BinOp::Lt => Ok(Const::Number(Number(
                        if child1val.as_number()?.0 < child2val.as_number()?.0 {
                            1.0
                        } else {
                            0.0
                        },
                    ))),
                    BinOp::Gt => Ok(Const::Number(Number(
                        if child1val.as_number()?.0 > child2val.as_number()?.0 {
                            1.0
                        } else {
                            0.0
                        },
                    ))),
                    BinOp::Le => Ok(Const::Number(Number(
                        if child1val.as_number()?.0 <= child2val.as_number()?.0 {
                            1.0
                        } else {
                            0.0
                        },
                    ))),
                    BinOp::Ge => Ok(Const::Number(Number(
                        if child1val.as_number()?.0 >= child2val.as_number()?.0 {
                            1.0
                        } else {
                            0.0
                        },
                    ))),
                    BinOp::Eq => Ok(Const::Number(Number(if child1val == child2val {
                        1.0
                    } else {
                        0.0
                    }))),
                    BinOp::Ne => Ok(Const::Number(Number(if child1val != child2val {
                        1.0
                    } else {
                        0.0
                    }))),
                }
            }
            Expression::UnOp(op, child) => {
                let child_val = child.evaluate(env);
                match op {
                    UnOp::Neg => Ok(Const::Number(Number(-child_val?.as_number()?.0))),
                    UnOp::Exists => Ok(Const::Number(Number(if child_val.is_ok() {
                        1.0
                    } else {
                        0.0
                    }))),
                }
            }
            Expression::Match {
                test,
                arms,
                default,
            } => {
                let expr_val = test.evaluate(env)?;
                let resulting_expr = arms.get(&expr_val).unwrap_or(default);
                resulting_expr.evaluate(env)
            }
        }
    }
}

impl Const {
    pub fn as_number(&self) -> Result<Number, String> {
        match self {
            Const::Number(n) => Ok(n.clone()),
            Const::String(s) => Err(format!("Expected number, found string \"{}\"", s)),
        }
    }

    // maybe we want this to be able to fail in the future
    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Const::Number(n) => Ok(n.0.to_string()),
            Const::String(s) => Ok(s.clone()), // ummmm
        }
    }

    pub fn from_num<N>(i: N) -> Const
    where
        N: Into<f64>,
    {
        Const::Number(Number(i.into()))
    }

    pub fn from_attr(a: &celeste::binel::BinElAttr) -> Const {
        match a {
            BinElAttr::Bool(b) => Const::from_num(if *b { 1 } else { 0 }),
            BinElAttr::Int(i) => Const::from_num(*i),
            BinElAttr::Float(f) => Const::from_num(*f),
            BinElAttr::Text(s) => Const::String(s.clone()),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_expr2() {
        assert_eq!(
            expression("-x * y"),
            Ok((
                "",
                Expression::BinOp(
                    BinOp::Mul,
                    Box::new((
                        Expression::UnOp(UnOp::Neg, Box::new(Expression::Atom("x".to_owned()))),
                        Expression::Atom("y".to_owned())
                    ))
                )
            ))
        );
    }

    #[test]
    fn test_expr3() {
        assert_eq!(
            expression("-x + y"),
            Ok((
                "",
                Expression::BinOp(
                    BinOp::Add,
                    Box::new((
                        Expression::UnOp(UnOp::Neg, Box::new(Expression::Atom("x".to_owned()))),
                        Expression::Atom("y".to_owned())
                    ))
                )
            ))
        );
    }

    #[test]
    fn test_match() {
        let expr = expression("match 1 + 1 { 2 => 'yeah', _ => 'what' }");
        assert!(expr.is_ok());
    }

    #[test]
    fn test_eval() {
        let expr = expression(
            "\
        match x + 1 {\
          1 => x / 0,\
          2 => -x,\
          3 => x * 2,\
          4 => 1 + '2',\
          5 => 1 - '2',\
          _ => 'foo'\
        }",
        );
        assert!(expr.is_ok());
        let expr = expr.unwrap().1;

        let mut env: HashMap<&str, Const> = HashMap::new();
        env.insert("x", Const::Number(Number(0f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(f64::NAN))));
        env.insert("x", Const::Number(Number(1f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(-1f64))));
        env.insert("x", Const::Number(Number(2f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(4f64))));
        env.insert("x", Const::Number(Number(3f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::String("12".to_owned())));
        env.insert("x", Const::Number(Number(4f64)));
        let res = expr.evaluate(&env);
        assert!(res.is_err());
        env.insert("x", Const::Number(Number(5f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::String("foo".to_owned())));
    }

    #[test]
    fn test_display() {
        let expr = expression(
            "\
        match x + 1 {\
          1 => x / 0,\
          2 => -x,\
          3 => x * 2,\
          4 => 1 + '2',\
          5 => 1 - '2',\
          _ => 'foo'\
        }",
        )
        .unwrap()
        .1;
        let expr_str = format!("{}", expr);
        let expr2 = expression(expr_str.as_str()).unwrap().1;
        assert_eq!(expr, expr2);
    }

    #[test]
    fn test_exists() {
        let expr = expression("?x").unwrap().1;
        let mut env: HashMap<&str, Const> = HashMap::new();
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(0.0))));
        env.insert("x", Const::Number(Number(0.0)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(1.0))));
    }

    #[test]
    fn test_empty_string() {
        let expr = expression("''");
        assert!(expr.is_ok());
        let res = expr.unwrap().1.evaluate(&HashMap::new());
        assert_eq!(res, Ok(Const::String("".to_owned())));
    }
}
