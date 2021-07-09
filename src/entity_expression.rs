use nom::{IResult, named, map_res, tuple, opt, tag, alt, delimited, recognize, one_of, many0, pair,
          character::complete::multispace0 as ws, is_not, number::complete::double, error::Error,
          preceded, separated_list0, separated_pair, is_a, complete, terminated, eof, do_parse,
          fold_many0, error::ErrorKind};
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone, PartialEq, Eq)] // WHY DO WE NEED CLONE OMG
pub enum Expression {
    Const(Const),
    Atom(String),
    BinOp(BinOp, Box<(Expression, Expression)>),
    UnOp(UnOp, Box<Expression>),
    Match { test: Box<Expression>, arms: HashMap<Const, Expression>, default: Box<Expression> }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnOp {
    Neg,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone)]
pub enum Const {
    Number(Number),
    String(String),
}

#[derive(Debug, Clone)]
pub struct Number(f64);

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

named!(num_lit<&str, Number>,
    map_res!(
        alt!(double | hex_num),
        |s: f64| -> Result<Number, Error<&str>> { Ok(Number(s)) }
    )
);

named!(hex_num<&str, f64>,
    map_res!(
        tuple!(
            opt!(alt!(tag!("-") | tag!("+"))),
            alt!(tag!("0x") | tag!("0X")),
            nom::character::complete::hex_digit1
        ),
        |t: (Option<&str>, &str, &str)| i64::from_str_radix(t.2, 16).map(
            |q: i64| q as f64 * if t.0 == Some("-") { -1f64 } else { 1f64 }
        )
    )
);

// TODO use escaped_transform
named!(string_lit_dquote<&str, &str>,
    delimited!(tag!("\""),is_not!("\""),tag!("\""))
);

named!(string_lit_squote<&str, &str>,
    delimited!(tag!("'"),is_not!("'"),tag!("'"))
);

named!(string_lit<&str, &str>,
    alt!(string_lit_dquote | string_lit_squote)
);

const IDENT_START_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const IDENT_CONT_CHARS: &str = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

named!(atom<&str, Expression>,
    map_res!(
        delimited!(
            ws,
            recognize!(pair!(
                one_of!(IDENT_START_CHARS),
                many0!(complete!(one_of!(IDENT_CONT_CHARS)))
            )),
            ws),
        |s: &str| -> Result<Expression, Error<&str>> { Ok(Expression::Atom(s.to_owned())) }
    )
);

named!(string_const<&str, Expression>,
    map_res!(
        delimited!(ws, string_lit, ws),
        |s: &str| -> Result<Expression, Error<&str>> { Ok(Expression::Const(Const::String(s.to_owned()))) }
    )
);

named!(num_const<&str, Expression>,
    map_res!(
        delimited!(ws, num_lit, ws),
        |s: Number| -> Result<Expression, Error<&str>> { Ok(Expression::Const(Const::Number(s))) }
    )
);

named!(parenthetical<&str, Expression>,
    delimited!(
        delimited!(ws, tag!("("), ws),
        expression_3,
        delimited!(ws, tag!(")"), ws)
    )
);

named!(match_case_const<&str, Option<Const>>,
    map_res!(
        alt!(string_const | num_const),
        |s: Expression| -> Result<Option<Const>, Error<&str>> { Ok(Some(match s { // oops I did not architect this correctly
            Expression::Const(c) => c,
            _ => unreachable!()
        })) }
    )
);

named!(match_case_default<&str, Option<Const>>,
    map_res!(
        delimited!(ws, tag!("_"), ws),
        |_| -> Result<Option<Const>, Error<&str>> { Ok(None) }
    )
);

named!(match_case<&str, Option<Const>>,
    alt!(match_case_const | match_case_default)
);

named!(match_arm<&str, (Option<Const>, Expression)>,
    separated_pair!(match_case, delimited!(ws, tag!("=>"), ws), expression_3)
);

fn construct_match_expr(test: Expression, arms_list: Vec<(Option<Const>, Expression)>) -> Result<Expression, ()> {
    let mut arms: HashMap<Const, Expression> = HashMap::new();
    let mut default: Option<Expression> = None;
    for (case, expr) in arms_list {
        match case {
            Some(real_case) => {
                if arms.contains_key(&real_case) {
                    return Err(())
                }
                arms.insert(real_case, expr);
            },
            None => {
                if default.is_some() {
                    return Err(())
                }
                default = Some(expr);
            }
        }
    }

    if default.is_none() {
        Err(())
    } else {
        Ok(Expression::Match {test: Box::new(test), arms, default: Box::new(default.unwrap())})
    }
}

named!(match_expr_with_errors<&str, Result<Expression, ()>>,
    do_parse!(
        delimited!(ws, tag!("match"), ws) >>
        test: expression_3 >>
        delimited!(ws, tag!("{"), ws) >>
        arms_list: separated_list0!(delimited!(ws, tag!(","), ws), match_arm) >>
        delimited!(ws, tag!("}"), ws) >>
        (construct_match_expr(test, arms_list))
    )
);

named!(match_expr<&str, Expression>,
    map_res!(
        match_expr_with_errors,
        |s: Result<Expression, ()>| -> Result<Expression, Error<&str>> {
            s.map_err(|_| Error { input: "what", code: ErrorKind::MapRes })
        }
    )
);

named!(expression_3<&str, Expression>,
    do_parse!(
        init: expression_2 >>
        res: fold_many0!(
            complete!(pair!(
                delimited!(ws, alt!(tag!("+") | tag!("-")), ws),
                expression_2
            )),
            init,
            |acc, next| Expression::BinOp(match next.0 {
                "+" => BinOp::Add,
                "-" => BinOp::Sub,
                _ => unreachable!(),
            }, Box::new((acc, next.1)))
        ) >>
        (res)
    )
);

named!(expression_2<&str, Expression>,
    do_parse!(
        init: expression_1 >>
        res: fold_many0!(
            complete!(pair!(
                delimited!(ws, alt!(tag!("*") | tag!("/") | tag!("%")), ws),
                expression_1
            )),
            init,
            |acc, next| Expression::BinOp(match next.0 {
                "*" => BinOp::Mul,
                "/" => BinOp::Div,
                "%" => BinOp::Mod,
                _ => unreachable!(),
            }, Box::new((acc, next.1)))
        ) >>
        (res)
    )
);

named!(expression_1<&str, Expression>,
    alt!(
        expression_0 |
        map_res!(
            tuple!(
                delimited!(ws, tag!("-"), ws),
                expression_1
            ), |s: (&str, Expression)| -> Result<Expression, Error<&str>> {
                Ok(Expression::UnOp(match s.0 {
                    "-" => UnOp::Neg,
                    _ => unreachable!(),
                }, Box::new(s.1)))
            }
        )
    )
);

named!(expression_0<&str, Expression>,
    // match must go first in the list since it could be interpreted as an atom
    alt!(match_expr | string_const | num_const | atom | parenthetical)
);

named!(expression<&str, Expression>,
    terminated!(expression_3, eof!())
);

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
        "can't serialize stuff yet!".serialize(s)
    }
}

impl Expression {
    pub fn mk_const(con: i32) -> Expression {
        Expression::Const(Const::Number(Number(con as f64)))
    }

    pub fn evaluate(&self, env: &HashMap<String, Const>) -> Result<Const, String> {
        match self {
            Expression::Const(c) => Ok(c.clone()),
            Expression::Atom(name) =>
                env.get(name)
                .map(|x| x.clone())
                .ok_or_else(|| format!("Name \"{}\" undefined", name)),
            Expression::BinOp(op, children) => {
                let child1val = children.as_ref().0.evaluate(env)?;
                let child2val = children.as_ref().1.evaluate(env)?;
                match op {
                    BinOp::Add => {
                        if let (&Const::Number(Number(n1)), &Const::Number(Number(n2))) = (&child1val, &child2val) {
                            Ok(Const::Number(Number(n1 + n2)))
                        } else {
                            Ok(Const::String(child1val.as_string()?.to_owned() + &child2val.as_string()?))
                        }
                    }
                    BinOp::Sub => Ok(Const::Number(Number(child1val.as_number()?.0 - child2val.as_number()?.0))),
                    BinOp::Mul => Ok(Const::Number(Number(child1val.as_number()?.0 * child2val.as_number()?.0))),
                    // division by zero can produce nan and that's okay (?)
                    BinOp::Div => Ok(Const::Number(Number(child1val.as_number()?.0 / child2val.as_number()?.0))),
                    BinOp::Mod => Ok(Const::Number(Number(child1val.as_number()?.0 % child2val.as_number()?.0))),
                }
            },
            Expression::UnOp(op, child) => {
                let child_val = child.evaluate(env)?;
                match op {
                    UnOp::Neg => Ok(Const::Number(Number(-child_val.as_number()?.0)))
                }
            },
            Expression::Match { test, arms, default } => {
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
            Const::String(s) => Err(format!("Expected number, found string \"{}\"", s))
        }
    }

    // maybe we want this to be able to fail in the future
    pub fn as_string(&self) -> Result<String, String> {
        match self {
            Const::Number(n) => Ok(n.0.to_string()),
            Const::String(s) => Ok(s.clone()), // ummmm
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
            Ok(("",
                Expression::BinOp(
                    BinOp::Mul,
                    Box::new((
                        Expression::UnOp(
                            UnOp::Neg,
                            Box::new(Expression::Atom("x".to_owned()))
                        ),
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
            Ok(("",
                Expression::BinOp(
                    BinOp::Add,
                    Box::new((
                        Expression::UnOp(
                            UnOp::Neg,
                            Box::new(Expression::Atom("x".to_owned()))
                        ),
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
        let expr = expression("\
        match x + 1 {\
          1 => x / 0,\
          2 => -x,\
          3 => x * 2,\
          4 => 1 + '2',\
          5 => 1 - '2',\
          _ => 'foo'\
        }");
        assert!(expr.is_ok());
        let expr = expr.unwrap().1;

        let mut env: HashMap<String, Const> = HashMap::new();
        env.insert("x".to_owned(), Const::Number(Number(0f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(f64::NAN))));
        env.insert("x".to_owned(), Const::Number(Number(1f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(-1f64))));
        env.insert("x".to_owned(), Const::Number(Number(2f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::Number(Number(4f64))));
        env.insert("x".to_owned(), Const::Number(Number(3f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::String("12".to_owned())));
        env.insert("x".to_owned(), Const::Number(Number(4f64)));
        let res = expr.evaluate(&env);
        assert!(res.is_err());
        env.insert("x".to_owned(), Const::Number(Number(5f64)));
        let res = expr.evaluate(&env);
        assert_eq!(res, Ok(Const::String("foo".to_owned())));
    }
}
