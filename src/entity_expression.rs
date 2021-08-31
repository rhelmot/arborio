use celeste::binel::BinElAttr;
use nom::branch::alt;
use nom::bytes::complete::{is_not, tag};
use nom::character::complete::{multispace1, one_of};
use nom::combinator::{complete, eof, map_res, opt, recognize};
use nom::error::ParseError;
use nom::multi::{fold_many0, many0, many0_count, many_till, separated_list0};
use nom::number::complete::hex_u32;
use nom::sequence::{delimited, pair, preceded, separated_pair, terminated, tuple};
use nom::Parser;
use nom::{
    character::complete::multispace0 as ws, error::Error, error::ErrorKind,
    number::complete::double, IResult,
};
use nom::{error, Err};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::num::ParseIntError;

#[derive(Debug, Clone, PartialEq)] // WHY DO WE NEED CLONE OMG
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl std::fmt::Display for BinOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnOp {
    Neg,
    Pos,
}

impl std::fmt::Display for UnOp {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                UnOp::Neg => "-",
                UnOp::Pos => "+",
            }
        )
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
                if !s.contains("\"") {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "\'{}\'", s)
                }
            }
        }
    }
}

impl From<Number> for Const {
    fn from(n: Number) -> Self {
        Self::Number(n)
    }
}

#[derive(Debug, Clone)]
pub struct Number(f64);

impl Number {
    pub fn to_int(&self) -> i32 {
        self.0 as i32
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
    map_res(
        alt((hex_num, double)),
        |s: f64| -> Result<Number, Error<&str>> { Ok(Number(s)) },
    )(input)
}

fn hex_num(input: &str) -> IResult<&str, f64> {
    map_res(
        separated_pair(
            opt(alt((tag("-"), tag("+")))),
            alt((tag("0x"), tag("0X"))),
            nom::character::complete::hex_digit1,
        ),
        |(sign, digits): (Option<&str>, _)| -> Result<f64, ParseIntError> {
            let number =
                i128::from_str_radix(&(sign.unwrap_or_default().to_owned() + digits), 16)? as f64;

            Ok(if sign == Some("-") { -number } else { number })
        },
    )(input)
}

// TODO use escaped_transform
fn string_lit_dquote(input: &str) -> IResult<&str, &str> {
    delimited(tag("\""), is_not("\""), tag("\""))(input)
}

fn string_lit_squote(input: &str) -> IResult<&str, &str> {
    delimited(tag("'"), is_not("'"), tag("'"))(input)
}

fn string_lit(input: &str) -> IResult<&str, &str> {
    alt((string_lit_dquote, string_lit_squote))(input)
}

const IDENT_START_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const IDENT_CONT_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn atom(input: &str) -> IResult<&str, Expression> {
    recognize(pair(
        one_of(IDENT_CONT_CHARS),
        many0(complete(one_of(IDENT_CONT_CHARS))),
    ))
    .map(|s: &str| Expression::Atom(s.to_owned()))
    .parse(input)
}

fn string_const(input: &str) -> IResult<&str, Expression> {
    string_lit
        .map(|s: &str| Expression::Const(Const::String(s.to_owned())))
        .parse(input)
}

fn num_const(input: &str) -> IResult<&str, Expression> {
    num_lit
        .map(|s: Number| Expression::Const(Const::Number(s)))
        .parse(input)
}

fn parenthetical(input: &str) -> IResult<&str, Expression> {
    delimited(
        terminated(tag("("), ws),
        expression_3,
        preceded(ws, tag(")")),
    )(input)
}

fn match_case_const(input: &str) -> IResult<&str, Option<Const>> {
    alt((string_const, num_const))
        .map(|s: Expression| {
            Some(if let Expression::Const(c) = s {
                c
            } else {
                unreachable!()
            })
        })
        .parse(input)
}

fn match_case_default(input: &str) -> IResult<&str, Option<Const>> {
    tag("_").map(|_| None).parse(input)
}

fn match_case(input: &str) -> IResult<&str, Option<Const>> {
    alt((match_case_const, match_case_default))(input)
}

fn match_arm(input: &str) -> IResult<&str, (Option<Const>, Expression)> {
    separated_pair(match_case, delimited(ws, tag("=>"), ws), expression_3)(input)
}

fn construct_match_expr(
    test: Expression,
    arms_list: Vec<(Option<Const>, Expression)>,
) -> Result<Expression, ()> {
    let mut arms: HashMap<Const, Expression> = HashMap::new();
    let mut default: Option<Expression> = None;
    for (case, expr) in arms_list {
        let old_value = match case {
            Some(real_case) => arms.insert(real_case, expr),
            None => default.replace(expr),
        };
        if old_value.is_some() {
            return Err(());
        }
    }

    let default = default.ok_or(())?;

    Ok(Expression::Match {
        test: Box::new(test),
        arms,
        default: Box::new(default),
    })
}

fn match_expr_with_errors(input: &str) -> IResult<&str, Result<Expression, ()>> {
    separated_pair(
        preceded(terminated(tag("match"), multispace1), expression_3),
        ws,
        delimited(
            terminated(tag("{"), ws),
            separated_list0(delimited(ws, tag(","), ws), match_arm),
            tuple((ws, opt(pair(tag(","), ws)), tag("}"))),
        ),
    )
    .map(|(test, arms)| construct_match_expr(test, arms))
    .parse(input)
}

fn match_expr(input: &str) -> IResult<&str, Expression> {
    map_res(
        match_expr_with_errors,
        |s: Result<Expression, ()>| -> Result<Expression, Error<&str>> {
            s.map_err(|_| Error {
                input: "what",
                code: ErrorKind::MapRes,
            })
        },
    )(input)
}

fn mut_ref_parser<P, I, O, E>(parser: &mut P) -> impl Parser<I, O, E> + '_
where
    P: Parser<I, O, E>,
{
    move |input| parser.parse(input)
}

fn fn_convert_mut<F: FnOnce(I) -> O, I, O>(f: F) -> impl FnMut(I) -> F::Output {
    let mut f = Some(f);
    move |i| (f.take().unwrap())(i)
}

fn left_binop<'a, B, O, T, F, Out>(
    mut base: B,
    mut operator: O,
    mut fold: F,
) -> impl FnMut(&'a str) -> IResult<&'a str, Out>
where
    B: Parser<&'a str, Out, nom::error::Error<&'a str>>,
    O: Parser<&'a str, T, error::Error<&'a str>>,
    F: FnMut(Out, T, Out) -> Out,
{
    move |input: &str| {
        let (mut input, accumulator) = base.parse(input)?;
        let mut accumulator = Some(accumulator);
        let mut pair = pair(delimited(ws, |i| operator.parse(i), ws), |i| base.parse(i));
        fold_many0(
            pair,
            move || accumulator.take().unwrap(),
            |lhs, (op, rhs)| fold(lhs, op, rhs),
        )(input)
    }
}

fn expression_3(input: &str) -> IResult<&str, Expression> {
    left_binop(
        expression_2,
        alt((tag("+").map(|_| BinOp::Add), tag("-").map(|_| BinOp::Sub))),
        |lhs, operator, rhs| Expression::BinOp(operator, Box::new((lhs, rhs))),
    )(input)
}

fn expression_2(input: &str) -> IResult<&str, Expression> {
    left_binop(
        expression_1,
        alt((
            tag("*").map(|_| BinOp::Mul),
            tag("/").map(|_| BinOp::Div),
            tag("%").map(|_| BinOp::Mod),
        )),
        |lhs, operator, rhs| Expression::BinOp(operator, Box::new((lhs, rhs))),
    )(input)
}

fn expression_1(input: &str) -> IResult<&str, Expression> {
    many_till(
        terminated(
            alt((tag("-").map(|_| UnOp::Neg), tag("+").map(|_| UnOp::Pos))),
            ws,
        ),
        expression_0,
    )
    .map(|(unaries, base)| {
        unaries
            .into_iter()
            .rev()
            .fold(base, |acc, unary| Expression::UnOp(unary, Box::new(acc)))
    })
    .parse(input)
}

fn expression_0(input: &str) -> IResult<&str, Expression> {
    // match must go first in the list since it could be interpreted as an atom
    alt((match_expr, string_const, num_const, atom, parenthetical))(input)
}

fn expression(input: &str) -> IResult<&str, Expression> {
    terminated(delimited(ws, expression_3, ws), eof)(input)
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
            panic!(); // ummmm how do you construct this kind of error
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
                .map(|x| x.clone())
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
                                child1val.as_string()?.to_owned() + &child2val.as_string()?,
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
                }
            }
            Expression::UnOp(op, child) => {
                let child_val = child.evaluate(env)?;
                match op {
                    UnOp::Neg => Ok(Const::Number(Number(-child_val.as_number()?.0))),
                    UnOp::Pos => Ok(child_val),
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
            BinElAttr::Bool(b) => Const::from(if *b { 1 } else { 0 }),
            BinElAttr::Int(i) => Const::from(*i),
            BinElAttr::Float(f) => Const::from(*f),
            BinElAttr::Text(s) => Const::String(s.clone()),
        }
    }
}

impl<N> From<N> for Const
where
    N: Into<f64>,
{
    fn from(n: N) -> Self {
        Const::Number(Number(n.into()))
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
}
