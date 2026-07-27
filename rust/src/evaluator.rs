//! Core LaTeX evaluation logic.
//!
//! Replace `evaluate_latex_impl` with your own Rust implementation.
//! The function receives a parsed request and must return LaTeX on success,
//! or `Err(...)` when the input cannot be evaluated.

use mathlex::{parse_latex, BinaryOp, ExprKind, Expression, MathFloat, ToLatex};
use serde::{Deserialize, Serialize};

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

    let expr = parse_latex(formula);

    if let Ok(expression) = expr {
        let output = evaluate(expression)?;

        return Ok(output.to_latex());
    }

    Err(format!("unsupported expression: {formula}"))
}

fn evaluate(expression: Expression) -> Result<Expression, String> {
    match expression.kind {
        ExprKind::Integer(_) | ExprKind::Float(_) => Ok(expression),
        ExprKind::Binary { op, left, right } => evaluate_binary(op, *left, *right),
        ExprKind::Function { name, args } => evaluate_function(name, args),
        ExprKind::Vector(vec) => {
            let mut evaluated_args = Vec::new();
            for arg in vec {
                evaluated_args.push(evaluate(arg)?);
            }
            Ok(Expression::new(ExprKind::Vector(evaluated_args)))
        }

        _ => Err(format!(
            "expression kind not recognized: {:?}",
            expression.kind
        )),
    }
}

fn evaluate_function(name: String, args: Vec<Expression>) -> Result<Expression, String> {
    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate(arg)?);
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
) -> Result<Expression, String> {
    match &operator {
        BinaryOp::Add => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                (ExprKind::Vector(lhs), ExprKind::Vector(rhs)) => {
                    let evaluated = lhs
                        .iter()
                        .cloned()
                        .zip(rhs.iter().cloned())
                        .map(|(l, r)| evaluate_binary(BinaryOp::Add, l, r))
                        .collect::<Result<Vec<_>, _>>()?;

                    Ok(Expression::new(ExprKind::Vector(evaluated)))
                }
                //ExprKind::Rational{numerator, denominator}  =>  Ok(Expression::new(ExprKind::Rational{numerator: Box::from(Expression::new(ExprKind::Binary { op: BinaryOp::Add, left: numerator, right: Box::from(Expression::new(ExprKind::Binary { op: BinaryOp::Mul, left: Box::from(left), right: denominator.clone() })) })), denominator })),
                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }
        BinaryOp::Sub => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }
        BinaryOp::Mul => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }
        BinaryOp::Div => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }
        BinaryOp::Pow => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }
        BinaryOp::Mod => {
            let left = evaluate(left)?;
            let right = evaluate(right)?;

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

                _ => Err(format!("Operator not recognized: {:?}", operator)),
            }
        }

        _ => Err(format!("Operator not recognized: {:?}", operator)),
    }
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
        };

        assert_eq!(evaluate_latex_impl(&request).unwrap(), "3");
    }

	#[test]
	fn adds_vectors() {
		let request = EvaluateRequest {
			formula: r"\begin{pmatrix}1 \\2 \\3\end{pmatrix}+\begin{pmatrix}2 \\3 \\4\end{pmatrix}".into(),
			previous_lines: vec![],
			approximate: false,
			precision: -1,
		};

		assert_eq!(evaluate_latex_impl(&request).unwrap(), r"\begin{pmatrix} 3 \\ 5 \\ 7 \end{pmatrix}");
	}
}
