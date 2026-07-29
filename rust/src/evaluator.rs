//! Core LaTeX evaluation logic.
//!
//! Replace `evaluate_latex_impl` with your own Rust implementation.
//! The function receives a parsed request and must return LaTeX on success,
//! or `Err(...)` when the input cannot be evaluated.

use mathlex::{parse_latex, parse_latex_lenient, BinaryOp, ExprKind, Expression, MathConstant, MathFloat, ToLatex, UnaryOp};
use serde::{Deserialize, Serialize};
use std::f64::consts::{E, PI};

/// Input passed from the Obsidian plugin to the Rust evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateRequest {
    /// The LaTeX expression to evaluate (trigger suffix already removed).
    pub formula: String,
    /// Earlier lines in the same math block (for variable definitions).
    #[serde(default)]
    pub previous_lines: Vec<String>,
    /// Whether the user triggered approximation mode (`\approx`).
    #[serde(default)]
    pub approximate: bool,
    /// Decimal precision for approximations (-1 means maximum precision).
    #[serde(default = "default_precision")]
    pub precision: i32,
    /// Whether the user triggered approximation mode (`\approx`).
    #[serde(default)]
    pub shift_for_exact: bool,
}

fn default_precision() -> i32 {
    -1
}

/// Replace this function with your LaTeX evaluation logic.
pub fn evaluate_latex_impl(request: &EvaluateRequest) -> Result<String, String> {
    let formula = request.formula.trim();

    if formula.is_empty() {
        return Err("empty formula".into());
    }

    let expr = parse_latex_lenient(formula);

    if let Some(expression) = expr.expression {
        let output = evaluate(expression, request.shift_for_exact)?;
        let output_string = output.to_latex();

        return Ok(round_expression(request, output_string));
    }

    Err(format!("unsupported expression: {formula}")) //todo output when errors are not 0
}
fn round_expression(request: &EvaluateRequest, value: String) -> String { //todo round vectors etc
    let precision = if request.approximate {
        request.precision
    } else {
        10
    };
    let n: Result<f64, _> = value.parse();
    if let Ok(number) = n {
        let factor = 10f64.powi(precision);
        let mut rounded = (number * factor).round() / factor;
        if rounded == 0.0 {
            rounded = 0.0; // removes the negative sign
        }
        return rounded.to_string();
    }

    value
}

fn evaluate(expression: Expression, exact: bool) -> Result<Expression, String> {
    match expression.clone().kind {
        ExprKind::Integer(_) | ExprKind::Float(_) => Ok(expression),
        ExprKind::Binary { op, left, right } => evaluate_binary(op, *left, *right, exact),
        ExprKind::Function { name, args } => {
            let output = evaluate_function(name, args, exact);
            if let Ok(output) = output {
                //if it's exact check to see if the function had a whole output otherwise keep the function how it was
                if exact {
                    if let ExprKind::Float(value) = output.kind {
                        if (value.value() % 1.0 < 1E-6) {
                            return Ok(Expression::integer(value.value() as i64));
                        }
                    }
                    Ok(expression)
                } else {
                    Ok(output)
                }
            } else {
                output
            }
        }
		ExprKind::CrossProduct { left, right } => evaluate_cross_product(left, right, exact),
        ExprKind::Constant(con) => {
            if !exact {
                evaluate_constant(con)
            } else {
                Ok(expression)
            }
        }
		ExprKind::Variable(name) => {
			if !exact {
				Ok(expression) //todo try to find it or something?
			}else{
				Ok(expression)
			}
		}
        ExprKind::Unary { op, operand } => evaluate_unary(op, *operand, exact),
        ExprKind::Vector(vec) => {
            let mut evaluated_args = Vec::new();
            for arg in vec {
                evaluated_args.push(evaluate(arg, exact)?);
            }
            Ok(Expression::new(ExprKind::Vector(evaluated_args)))
        }

        _ => Err(format!(
            "expression kind not recognized: {:?}",
            expression.kind
        )),
    }
}

fn evaluate_cross_product(left: Box<Expression>, right: Box<Expression>, exact: bool) -> Result<Expression, String> {
	let left = evaluate(*left, exact)?;
	let right = evaluate(*right, exact)?;
	match (&left.kind, &right.kind) {
		//just use normal multiplication on numbers
		(ExprKind::Integer(_) | ExprKind::Float(_), ExprKind::Integer(_)| ExprKind::Float(_)) => evaluate_binary(BinaryOp::Mul, left, right, exact),

		(ExprKind::Vector(v1), ExprKind::Vector(v2)) => {
			if v1.len() == 3 && v2.len() == 3 {
				let mut output = Vec::new();
				output.push(evaluate_binary(BinaryOp::Sub, evaluate_binary(BinaryOp::Mul, v1[1].clone(), v2[2].clone(), exact)?, evaluate_binary(BinaryOp::Mul, v1[2].clone(), v2[1].clone(), exact)?, exact)?);
				output.push(evaluate_binary(BinaryOp::Sub, evaluate_binary(BinaryOp::Mul, v1[2].clone(), v2[0].clone(), exact)?, evaluate_binary(BinaryOp::Mul, v1[0].clone(), v2[2].clone(), exact)?, exact)?);
				output.push(evaluate_binary(BinaryOp::Sub, evaluate_binary(BinaryOp::Mul, v1[0].clone(), v2[1].clone(), exact)?, evaluate_binary(BinaryOp::Mul, v1[1].clone(), v2[0].clone(), exact)?, exact)?);

				Ok(Expression::new(ExprKind::Vector(output)))
			}else {
				Err("Cross product only works on vectors of length 3".into())
			}
		}
		_ => Err("Cross product does not exist for this data type".into()),
	}
}

fn evaluate_unary(op: UnaryOp, operand: Expression, exact: bool) -> Result<Expression, String> {
    match op {
        UnaryOp::Neg => Ok(evaluate_binary(
            BinaryOp::Sub,
            Expression::integer(0),
            operand,
            exact,
        )?),
        UnaryOp::Pos => Ok(operand),
        UnaryOp::Factorial => {
            if let ExprKind::Integer(int) = operand.kind {
                if int < 0 {
                    Err("Can't factorial negative".into())
                } else {
                    let mut total = 1;
                    for n in 0..int {
                        total *= n;
                    }
                    Ok(Expression::integer(total))
                }
            } else {
                Err("Could not evaluate factorial number".into())
            }
        }
        UnaryOp::Transpose => Err("Transpose not implemented".into()),
    }
}

fn evaluate_constant(constant: MathConstant) -> Result<Expression, String> {
    match constant {
        MathConstant::Pi => Ok(Expression::float(MathFloat::new(PI))),
        MathConstant::E => Ok(Expression::float(MathFloat::new(E))),

        MathConstant::Infinity => Ok(Expression::float(MathFloat::new(f64::INFINITY))),
        MathConstant::NegInfinity => Ok(Expression::float(MathFloat::new(f64::NEG_INFINITY))),
        MathConstant::NaN => Ok(Expression::float(MathFloat::new(f64::NAN))),
        _ => Err(format!("constant not recognized: {:?}", constant)),
    }
}

fn evaluate_function(
    name: String,
    args: Vec<Expression>,
    exact: bool,
) -> Result<Expression, String> {
    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate(arg, false)?);
    }
    //single argument functions
    if evaluated_args.len() == 1 {
        let value: f64 = match evaluated_args[0].kind {
            ExprKind::Integer(i) => i as f64,
            ExprKind::Float(f) => f.value(),
            _ => return Err("Function called with non-integer args".to_owned()),
        };

        return match name.as_str() {
            "cos" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.cos(),
            )))),
            "acos" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.acos(),
            )))),
            "cosh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.cosh(),
            )))),
            "acosh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.acosh(),
            )))),
            "sin" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.sin(),
            )))),
            "asin" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.asin(),
            )))),
            "sinh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.sinh(),
            )))),
            "asinh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.asinh(),
            )))),
            "tan" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.tan(),
            )))),
            "atan" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.atan(),
            )))),
            "tanh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.tanh(),
            )))),
            "atanh" => Ok(Expression::new(ExprKind::Float(MathFloat::new(
                value.atanh(),
            )))),
            "log" | "ln" => Ok(Expression::new(ExprKind::Float(MathFloat::new(value.ln())))),
            _ => return Err("Function called with non-function name".to_owned()),
        };
    }
    Err(format!("Function {} not found", name))
}

fn evaluate_binary(
    operator: BinaryOp,
    left: Expression,
    right: Expression,
    exact: bool,
) -> Result<Expression, String> {
    match &operator {
        BinaryOp::Add => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::new(ExprKind::Integer(lhs + rhs)))
                }
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 + rhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*rhs as f64 + lhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() + rhs.value())),
                )),

                (ExprKind::Vector(lhs), ExprKind::Vector(rhs)) => {
                    operate_on_vector(lhs, rhs, operator, exact)
                }

                //ExprKind::Rational{numerator, denominator}  =>  Ok(Expression::new(ExprKind::Rational{numerator: Box::from(Expression::new(ExprKind::Binary { op: BinaryOp::Add, left: numerator, right: Box::from(Expression::new(ExprKind::Binary { op: BinaryOp::Mul, left: Box::from(left), right: denominator.clone() })) })), denominator })),
                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Sub => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::new(ExprKind::Integer(lhs - rhs)))
                }
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 - rhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() - *rhs as f64)),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() - rhs.value())),
                )),

                (ExprKind::Vector(lhs), ExprKind::Vector(rhs)) => {
                    operate_on_vector(lhs, rhs, operator, exact)
                }

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Mul => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::new(ExprKind::Integer(lhs * rhs)))
                }
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 * rhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() * *rhs as f64)),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() * rhs.value())),
                )),

				(ExprKind::Vector(lhs), ExprKind::Vector(rhs)) => {
					operate_on_vector(lhs, rhs, operator, exact)
				}
				(ExprKind::Integer(_), ExprKind::Vector(vec)) | (ExprKind::Float(_), ExprKind::Vector(vec))=> {
					let new = vec.into_iter().cloned().map(|i| evaluate_binary(BinaryOp::Mul, left.clone(), i, exact)) .collect::<Result<Vec<_>, _>>()?;
					Ok(Expression::new(ExprKind::Vector(new)))
				}
				(ExprKind::Vector(vec),ExprKind::Integer(_))| (ExprKind::Vector(vec),ExprKind::Float(_))=> {
					let new = vec.into_iter().cloned().map(|i| evaluate_binary(BinaryOp::Mul, i, right.clone(), exact)) .collect::<Result<Vec<_>, _>>()?;
					Ok(Expression::new(ExprKind::Vector(new)))
				}

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Div => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 / *rhs as f64)),
                )),
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 / rhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() / *rhs as f64)),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() / rhs.value())),
                )),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Pow => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::new(ExprKind::Integer(lhs.pow(*rhs as u32))))
                }
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new((*lhs as f64).powf(rhs.value()))),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value().powf(*rhs as f64))),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value().powf(rhs.value()))),
                )),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Mod => {
            let left = evaluate(left, exact)?;
            let right = evaluate(right, exact)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::new(ExprKind::Integer(lhs % rhs)))
                }
                (ExprKind::Integer(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(*lhs as f64 % rhs.value())),
                )),
                (ExprKind::Float(lhs), ExprKind::Integer(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() % *rhs as f64)),
                )),
                (ExprKind::Float(lhs), ExprKind::Float(rhs)) => Ok(Expression::new(
                    ExprKind::Float(MathFloat::new(lhs.value() % rhs.value())),
                )),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }

        _ => simplify_or_error(left, right, operator, exact),
    }
}
/// show error if you can't find exact else best result when just simplifying
fn simplify_or_error(
    left: Expression,
    right: Expression,
    op: BinaryOp,
    simplifying: bool,
) -> Result<Expression, String> {
    if simplifying {
        Ok(Expression::new(ExprKind::Binary {
            op,
            left: Box::from(left),
            right: Box::from(right),
        }))
    } else {
        Err(format!("Operator not recognized: {:?}", op))
    }
}

fn operate_on_vector(
    lhs: &Vec<Expression>,
    rhs: &Vec<Expression>,
    op: BinaryOp,
    exact: bool,
) -> Result<Expression, String> {
    let evaluated = lhs
        .iter()
        .cloned()
        .zip(rhs.iter().cloned())
        .map(|(l, r)| evaluate_binary(op, l, r, exact))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Expression::new(ExprKind::Vector(evaluated)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_simple_integers() {
        let request = EvaluateRequest {
            formula: "1+2".into(),
            previous_lines: vec![],
            approximate: false,
            precision: -1,
            shift_for_exact: false,
        };

        assert_eq!(evaluate_latex_impl(&request).unwrap(), "3");
    }

    #[test]
    fn adds_vectors() {
        let request = EvaluateRequest {
            formula: r"\begin{pmatrix}1 \\2 \\3\end{pmatrix}+\begin{pmatrix}2 \\3 \\4\end{pmatrix}"
                .into(),
            previous_lines: vec![],
            approximate: false,
            precision: -1,
            shift_for_exact: false,
        };

        assert_eq!(
            evaluate_latex_impl(&request).unwrap(),
            r"\begin{pmatrix} 3 \\ 5 \\ 7 \end{pmatrix}"
        );
    }
    #[test]
    fn functions_and_constants() {
        let request = EvaluateRequest {
            formula: r"\sin(2\pi)".into(),
            previous_lines: vec![],
            approximate: false,
            precision: -1,
            shift_for_exact: false,
        };

        assert_eq!(evaluate_latex_impl(&request).unwrap(), "0"); //todo this works but rounding?
    }
}
