use {crate::syntax::ast::Expression, super::{Builder, Value}};

/// Lower an AST expression to IR instructions.
pub fn lower_expression(b: &mut Builder, expression: &Expression) -> Value
{
    match expression {

        Expression::NumericAdd{left, plus_sign, right} => {
            let left = lower_expression(b, left);
            let right = lower_expression(b, right);
            b.set_location(*plus_sign);
            b.build_numeric_add(left, right)
        },

        Expression::StringConcatenate{left, tilde, right} => {
            let left = lower_expression(b, left);
            let right = lower_expression(b, right);
            b.set_location(*tilde);
            b.build_string_concatenate(left, right)
        },

        Expression::StringLiteral{location: _, value} =>
            Value::String(value.clone()),

    }
}
