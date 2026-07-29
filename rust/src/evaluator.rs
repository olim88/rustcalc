//! Core LaTeX evaluation logic.
//!
//! Replace `evaluate_latex_impl` with your own Rust implementation.
//! The function receives a parsed request and must return LaTeX on success,
//! or `Err(...)` when the input cannot be evaluated.

use mathlex::ExprKind::Exists;
use mathlex::{
    parse_latex, parse_latex_equation_system, parse_latex_lenient, parse_system, BinaryOp,
    ExprKind, Expression, MathConstant, MathFloat, ParseResult, ToLatex, UnaryOp,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f64::consts::{E, PI};
use std::iter::Map;

/// Input passed from the Obsidian plugin to the Rust evaluator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluateRequest {
    /// The LaTeX expression to evaluate (trigger suffix already removed).
    pub formula: String,
    /// Earlier lines  (for variable definitions).
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
    //find veriables from previces lines

    let vars: HashMap<String, Expression> = exstract_vars(&request.previous_lines);

    let expr = parse_latex_lenient(formula);

    if let Some(expression) = expr.expression {
        let output = evaluate(expression, request.shift_for_exact, &vars)?;
        let output_string = output.to_latex();

        return Ok(round_expression(request, output_string));
    }

    Err(format!("unsupported expression: {formula}")) //todo output when errors are not 0
}

/// Looks at previous equations in the document and if any of them are defining a constant save them for later use
fn exstract_vars(lines: &Vec<String>) -> HashMap<String, Expression> {
    let vars_equations = parse_latex_equation_system(lines.join(";").as_str());
    match vars_equations {
        Ok(exps) => {
            let mut vars = HashMap::new();
            for exp in exps {
                if let ExprKind::Equation { left, right } = exp.kind {
                    if let ExprKind::Variable(var) = left.kind {
                        vars.insert(var, *right);
                    }
                }
            }

            vars
        }
        Err(_) => HashMap::default(),
    }
}

fn round_expression(request: &EvaluateRequest, value: String) -> String {
    //todo round vectors etc
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

fn evaluate(
    expression: Expression,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    match expression.clone().kind {
        ExprKind::Integer(_) | ExprKind::Float(_) => Ok(expression),
        ExprKind::Binary { op, left, right } => evaluate_binary(op, *left, *right, exact, vars),
        ExprKind::Function { name, args } => {
            let output = evaluate_function(name, args, exact, vars);
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
        ExprKind::CrossProduct { left, right } => evaluate_cross_product(left, right, exact, vars),
        ExprKind::Gradient { expr } => evaluate_gradient(*expr, exact, vars),
        ExprKind::Curl { field } => evaluate_curl(*field, exact, vars),
        ExprKind::Derivative { expr, var, order } => {
            evaluate_derivative(expr, &var, order, exact, vars)
        } //todo method for derivatives
        ExprKind::Differential { var } => Ok(expression), //todo method for Differential
        ExprKind::Constant(con) => {
            if !exact {
                evaluate_constant(con)
            } else {
                Ok(expression)
            }
        }
        ExprKind::Variable(name) => evaluate_variable(&name, exact, vars),
        ExprKind::Unary { op, operand } => evaluate_unary(op, *operand, exact, vars),
        ExprKind::Vector(vec) => {
            let mut evaluated_args = Vec::new();
            for arg in vec {
                evaluated_args.push(evaluate(arg, exact, vars)?);
            }
            Ok(Expression::new(ExprKind::Vector(evaluated_args)))
        }

        _ => Err(format!(
            "expression kind not recognized: {:?}",
            expression.kind
        )),
    }
}

fn evaluate_derivative(
    expression: Box<Expression>,
    var: &String,
    order: u32,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    let expression = evaluate(*expression, exact, vars)?;
    //if does not contain variable it will be 0 (if a variable is un definned it could be dependent on var so this only works for
    if expression.find_variables().is_empty() {
        Ok(Expression::integer(0))
    } else if let ExprKind::Binary { op, left, right } = expression.clone().kind {
        //split terms and differentiate them individually
        if op == BinaryOp::Add || op == BinaryOp::Sub {
            evaluate_binary(
                op,
                evaluate_derivative(left, &var, order, exact, vars)?,
                evaluate_derivative(right, &var, order, exact, vars)?,
                exact,
                vars,
            )
        } else if op == BinaryOp::Pow && !right.contains_variable(var) {
            let power = right;
            Ok(derivative(*left, *power, exact, vars)?)
        } else if op == BinaryOp::Mul && !left.contains_variable(var) {
            Ok(evaluate_binary(
                BinaryOp::Mul,
                *left,
                evaluate_derivative(right, &var, order, exact, vars)?,
                exact,
                vars,
            )?)
        } else {
            Ok(ExprKind::Derivative {
                expr: Box::from(expression),
                var: var.clone(),
                order,
            }
            .into())
        }
    } else if let ExprKind::Variable(var2) = expression.clone().kind {
        if *var == var2 {
            Ok(derivative(expression, Expression::integer(1), exact, vars)?)
        } else {
            Ok(ExprKind::Derivative {
                expr: Box::from(expression),
                var: var.clone(),
                order,
            }
            .into())
        }
    } else {
        if !exact {
            Err(format!(
                "Can't find {} deriviatve of: {:?}",
                var, expression.kind
            ))
        } else {
            Ok(ExprKind::Derivative {
                expr: Box::from(expression),
                var: var.clone(),
                order,
            }
            .into())
        }
    }
}

fn derivative(
    term: Expression,
    power: Expression,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    evaluate_binary(
        BinaryOp::Mul,
        power.clone(),
        evaluate_binary(
            BinaryOp::Pow,
            term,
            evaluate_binary(BinaryOp::Sub, power, Expression::integer(1), exact, vars)?,
            exact,
            vars,
        )?,
        exact,
        vars,
    )
}

fn evaluate_variable(
    name: &String,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    match vars.get(name) {
        Some(exp) => Ok(evaluate(exp.clone(), true, &vars)?),
        None => Ok(Expression::new(ExprKind::Variable(name.clone()))),
    }
}

/// Evaluate grad of an expression. Assumes that we are in Cartesian coords
fn evaluate_gradient(
    expression: Expression,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    let mut elements = Vec::new();
    let expression = evaluate(expression, exact, &vars)?;
    elements.push(evaluate(
        Expression::new(ExprKind::Derivative {
            expr: Box::from(expression.clone()),
            var: String::from("x"),
            order: 1,
        }),
        exact,
        &vars,
    )?);
    elements.push(evaluate(
        Expression::new(ExprKind::Derivative {
            expr: Box::from(expression.clone()),
            var: String::from("y"),
            order: 1,
        }),
        exact,
        &vars,
    )?);
    elements.push(evaluate(
        Expression::new(ExprKind::Derivative {
            expr: Box::from(expression.clone()),
            var: String::from("z"),
            order: 1,
        }),
        exact,
        &vars,
    )?);

    Ok(Expression::vector(elements))
}
/// Evaluate curl of a vector. Assumes that we are in Cartesian coords
fn evaluate_curl(
    expression: Expression,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    if let ExprKind::Vector(vec) = expression.clone().kind {
        let mut output = Vec::new();
        output.push(evaluate_binary(
            BinaryOp::Sub,
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[2].clone(), exact, vars)?),
                var: String::from("y"),
                order: 1,
            }
            .into(),
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[1].clone(), exact, vars)?),
                var: String::from("z"),
                order: 1,
            }
            .into(),
            exact,
            vars,
        )?);
        output.push(evaluate_binary(
            BinaryOp::Sub,
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[0].clone(), exact, vars)?),
                var: String::from("z"),
                order: 1,
            }
            .into(),
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[2].clone(), exact, vars)?),
                var: String::from("x"),
                order: 1,
            }
            .into(),
            exact,
            vars,
        )?);
        output.push(evaluate_binary(
            BinaryOp::Sub,
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[1].clone(), exact, vars)?),
                var: String::from("x"),
                order: 1,
            }
            .into(),
            ExprKind::Derivative {
                expr: Box::from(evaluate(vec[0].clone(), exact, vars)?),
                var: String::from("y"),
                order: 1,
            }
            .into(),
            exact,
            vars,
        )?);
        Ok(Expression::vector(output))
    } else {
        Err("expression dose not support curl".to_string())
    }
}

fn evaluate_cross_product(
    left: Box<Expression>,
    right: Box<Expression>,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    let left = evaluate(*left, exact, vars)?;
    let right = evaluate(*right, exact, vars)?;
    match (&left.kind, &right.kind) {
        //just use normal multiplication on numbers
        (ExprKind::Integer(_) | ExprKind::Float(_), ExprKind::Integer(_) | ExprKind::Float(_)) => {
            evaluate_binary(BinaryOp::Mul, left, right, exact, vars)
        }

        (ExprKind::Vector(v1), ExprKind::Vector(v2)) => {
            if v1.len() == 3 && v2.len() == 3 {
                let mut output = Vec::new();
                output.push(evaluate_binary(
                    BinaryOp::Sub,
                    evaluate_binary(BinaryOp::Mul, v1[1].clone(), v2[2].clone(), exact, vars)?,
                    evaluate_binary(BinaryOp::Mul, v1[2].clone(), v2[1].clone(), exact, vars)?,
                    exact,
                    vars,
                )?);
                output.push(evaluate_binary(
                    BinaryOp::Sub,
                    evaluate_binary(BinaryOp::Mul, v1[2].clone(), v2[0].clone(), exact, vars)?,
                    evaluate_binary(BinaryOp::Mul, v1[0].clone(), v2[2].clone(), exact, vars)?,
                    exact,
                    vars,
                )?);
                output.push(evaluate_binary(
                    BinaryOp::Sub,
                    evaluate_binary(BinaryOp::Mul, v1[0].clone(), v2[1].clone(), exact, vars)?,
                    evaluate_binary(BinaryOp::Mul, v1[1].clone(), v2[0].clone(), exact, vars)?,
                    exact,
                    vars,
                )?);

                Ok(Expression::vector(output))
            } else {
                Err("Cross product only works on vectors of length 3".into())
            }
        }
        _ => Err("Cross product does not exist for this data type".into()),
    }
}

fn evaluate_unary(
    op: UnaryOp,
    operand: Expression,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    match op {
        UnaryOp::Neg => Ok(evaluate_binary(
            BinaryOp::Sub,
            Expression::integer(0),
            operand,
            exact,
            vars,
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
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate(arg, false, vars)?);
    }
    //single argument functions
    if evaluated_args.len() == 1 {
        let value: f64 = match evaluated_args[0].kind {
            ExprKind::Integer(i) => i as f64,
            ExprKind::Float(f) => f.value(),
            _ => return Err("Function called with non-integer args".to_owned()),
        };

        return match name.as_str() {
            "cos" => Ok(ExprKind::Float(MathFloat::new(value.cos())).into()),
            "acos" => Ok(ExprKind::Float(MathFloat::new(value.acos())).into()),
            "cosh" => Ok(ExprKind::Float(MathFloat::new(value.cosh())).into()),
            "acosh" => Ok(ExprKind::Float(MathFloat::new(value.acosh())).into()),
            "sin" => Ok(ExprKind::Float(MathFloat::new(value.sin())).into()),
            "asin" => Ok(ExprKind::Float(MathFloat::new(value.asin())).into()),
            "sinh" => Ok(ExprKind::Float(MathFloat::new(value.sinh())).into()),
            "asinh" => Ok(ExprKind::Float(MathFloat::new(value.asinh())).into()),
            "tan" => Ok(ExprKind::Float(MathFloat::new(value.tan())).into()),
            "atan" => Ok(ExprKind::Float(MathFloat::new(value.atan())).into()),
            "tanh" => Ok(ExprKind::Float(MathFloat::new(value.tanh())).into()),
            "atanh" => Ok(ExprKind::Float(MathFloat::new(value.atanh())).into()),
            "log" | "ln" => Ok(ExprKind::Float(MathFloat::new(value.ln())).into()),
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
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    match &operator {
        BinaryOp::Add => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

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
                    operate_on_vector(lhs, rhs, operator, exact, vars)
                }
                (lhs, _) if is_zero(lhs) => Ok(right),
                (_, rhs) if is_zero(rhs) => Ok(left),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Sub => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

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
                    operate_on_vector(lhs, rhs, operator, exact, vars)
                }
                (_, rhs) if is_zero(rhs) => Ok(left),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Mul => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

            match (&left.kind, &right.kind) {
                //basic values
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(ExprKind::Integer(lhs * rhs).into())
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
                //vector dot product
                (ExprKind::Vector(lhs), ExprKind::Vector(rhs)) => {
                    operate_on_vector(lhs, rhs, operator, exact, vars)
                }
                // vector by constant
                (ExprKind::Integer(_), ExprKind::Vector(vec))
                | (ExprKind::Float(_), ExprKind::Vector(vec)) => {
                    let new = vec
                        .into_iter()
                        .cloned()
                        .map(|i| evaluate_binary(BinaryOp::Mul, left.clone(), i, exact, vars))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Expression::new(ExprKind::Vector(new)))
                }
                (ExprKind::Vector(vec), ExprKind::Integer(_))
                | (ExprKind::Vector(vec), ExprKind::Float(_)) => {
                    let new = vec
                        .into_iter()
                        .cloned()
                        .map(|i| evaluate_binary(BinaryOp::Mul, i, right.clone(), exact, vars))
                        .collect::<Result<Vec<_>, _>>()?;
                    Ok(Expression::new(ExprKind::Vector(new)))
                }
                //multiplying by 0 is 0
                (lhs, _) if is_zero(lhs) => Ok(Expression::integer(0)),
                (_, rhs) if is_zero(rhs) => Ok(Expression::integer(0)),

                //if multiplying by another multiplication try to simplify by multiplying first value only. Is this good? or just a bodge
                (
                    _,
					ExprKind::Binary {
						op,
						left: sub_left,
						right: sub_right,
					}
                ) if *op == BinaryOp::Mul => Ok(ExprKind::Binary {
                    op: BinaryOp::Mul,
                    left: Box::from(evaluate_binary(
                        BinaryOp::Mul,
                        left.clone(),
                        *sub_left.clone(),
                        exact,
                        vars,
                    )?),
                    right: Box::from(*sub_right.clone()),
                }
                .into()),

                //if the same simplify using power of 2 else leave it
                (lhs, rhs) => {
                    if lhs.eq(rhs) {
                        evaluate_binary(BinaryOp::Pow, left, Expression::integer(2), exact, vars)
                    } else {
                        simplify_or_error(left, right, operator, exact)
                    }
                }
            }
        }
        BinaryOp::Div => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

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
                (lhs, _) if is_zero(lhs) => Ok(Expression::integer(0)),
                (_, rhs) if is_zero(rhs) => Err("Can't divide by zero".to_owned()),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Pow => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

            match (&left.kind, &right.kind) {
                (ExprKind::Integer(lhs), ExprKind::Integer(rhs)) => {
                    Ok(Expression::integer(lhs.pow(*rhs as u32)))
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

                (_, rhs) if is_zero(rhs) => Ok(Expression::integer(1)),
                (_, ExprKind::Integer(rhs)) if *rhs == 1 => Ok(left),

                //simplify powers of powers
                (
                    ExprKind::Binary {
                        op,
                        left: sub_left,
                        right: sub_right,
                    },
                    _,
                ) if *op == BinaryOp::Pow => Ok(ExprKind::Binary {
                    op: BinaryOp::Pow,
                    left: sub_left.clone(),
                    right: Box::from(evaluate_binary(
                        BinaryOp::Mul,
                        *sub_right.clone(),
                        right,
                        exact,
                        vars,
                    )?),
                }
                .into()),

                _ => simplify_or_error(left, right, operator, exact),
            }
        }
        BinaryOp::Mod => {
            let left = evaluate(left, exact, vars)?;
            let right = evaluate(right, exact, vars)?;

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

fn is_zero(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Integer(n) => *n == 0,
        ExprKind::Float(f) => f.value() == 0.0,
        _ => false,
    }
}

fn operate_on_vector(
    lhs: &Vec<Expression>,
    rhs: &Vec<Expression>,
    op: BinaryOp,
    exact: bool,
    vars: &HashMap<String, Expression>,
) -> Result<Expression, String> {
    let evaluated = lhs
        .iter()
        .cloned()
        .zip(rhs.iter().cloned())
        .map(|(l, r)| evaluate_binary(op, l, r, exact, vars))
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

        assert_eq!(evaluate_latex_impl(&request).unwrap(), "0");
    }
    #[test]
    fn text_cross_product_and_variables() {
        let request = EvaluateRequest {
			formula: r"\begin{pmatrix}a_{1} \\  a_{2} \\  a_{3} \end{pmatrix}\times \begin{pmatrix}b_{1} \\  b_{2} \\  b_{3}\end{pmatrix}"
				.into(),
			previous_lines: vec![],
			approximate: false,
			precision: -1,
			shift_for_exact: true,
		};

        assert_eq!(
            evaluate_latex_impl(&request).unwrap(),
            r"\begin{pmatrix} a_2 \cdot b_3 - a_3 \cdot b_2 \\ a_3 \cdot b_1 - a_1 \cdot b_3 \\ a_1 \cdot b_2 - a_2 \cdot b_1 \end{pmatrix}"
        );
    }

    #[test]
    fn variables_with_definitions() {
        let request = EvaluateRequest {
            formula: r"x".into(),
            previous_lines: vec!["x=5+2".into()],
            approximate: false,
            precision: -1,
            shift_for_exact: false,
        };

        assert_eq!(evaluate_latex_impl(&request).unwrap(), "7");
    }
    #[test]
    fn curl_operator_cartesian() {
        let request = EvaluateRequest {
            formula: r"\nabla \times \begin{pmatrix}F_{1} \\  F_{2} \\  F_{3}\end{pmatrix}".into(),
            previous_lines: vec![],
            approximate: false,
            precision: -1,
            shift_for_exact: true,
        };

        assert_eq!(
            evaluate_latex_impl(&request).unwrap(),
            r"\begin{pmatrix} \frac{d}{dy}F_3 - \frac{d}{dz}F_2 \\ \frac{d}{dz}F_1 - \frac{d}{dx}F_3 \\ \frac{d}{dx}F_2 - \frac{d}{dy}F_1 \end{pmatrix}"
        );
    }

	#[test]
	fn derivatives() {
		let request = EvaluateRequest {
			formula: r"\frac{d}{dx}(2x^{10}+ 0.5x^{2}+ 12231+y)".into(),
			previous_lines: vec![],
			approximate: false,
			precision: -1,
			shift_for_exact: true,
		};

		assert_eq!(evaluate_latex_impl(&request).unwrap(), r"20 \cdot x^{9} + 1 \cdot x + \frac{d}{dx}y");
	}
}
