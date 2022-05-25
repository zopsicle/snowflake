//! Convert abstract syntax trees to bytecode.

pub use self::{error::*, symbol::*};

use {
    crate::{
        syntax::{
            ast::*,
            lex::Lexer,
            location::Location,
            parse::{Arenas, parse_unit},
        },
        unstable::{
            bytecode::{
                Builder,
                Constant,
                Instruction,
                Register,
                Unit,
                Verified,
            },
            value::Value,
        },
        util::try_new_cyclic,
    },
    self::collect_definitions::*,
    std::{collections::HashMap, path::PathBuf, sync::{Arc, Weak}},
    thiserror::Error,
};

pub mod symbol;

mod collect_definitions;
mod error;

/* -------------------------------------------------------------------------- */
/*                               Compiling units                              */
/* -------------------------------------------------------------------------- */

/// Compile a unit given its source code.
pub fn compile_unit_from_source(pathname: PathBuf, source: &str)
    -> Result<Arc<Unit>>
{
    Arenas::with(|arenas| {
        let mut lexer = Lexer::new(source).peekable();
        let definitions = parse_unit(arenas, &mut lexer)?;
        compile_unit(pathname, &definitions)
    })
}

/// Compile a unit given its definitions.
pub fn compile_unit(pathname: PathBuf, definitions: &[Definition])
    -> Result<Arc<Unit>>
{
    try_new_cyclic(|unit| {
        // Collect information about all definitions.
        let CollectedDefinitions{constants_allocated, globals} =
            collect_definitions(definitions)?;

        // Pre-allocate storage for constants that store globals.
        // These will be overwritten whilst compiling the definitions.
        let mut constants = vec![Value::undef(); constants_allocated as usize];

        // Compile each definition.
        let mut init_phasers = Vec::new();
        for definition in definitions {
            compile_definition(
                unit.clone(),
                &mut constants,
                &mut init_phasers,
                &globals,
                definition,
            )?;
        }

        Ok(Unit{pathname, constants, init_phasers, globals})
    })
}

/* -------------------------------------------------------------------------- */
/*                           Compiling unit elements                          */
/* -------------------------------------------------------------------------- */

// The below code uses abbreviations for ubiquitious parameters.
// The abbreviations are as follows:
//
//  - I for Instruction
//  - u for unit
//  - c for constants

use Instruction as I;

/// Compile a definition.
///
/// This initializes any constants pre-allocated for the definition.
/// It may also define new constants, depending on the definition.
fn compile_definition(
    u: Weak<Unit>,
    c: &mut Vec<Value>,
    init_phasers: &mut Vec<Constant>,
    globals: &HashMap<Arc<str>, Constant>,
    definition: &Definition,
) -> Result<()>
{
    match definition {

        Definition::InitPhaser{body, right_curly_bracket, ..} => {
            let end_location = *right_curly_bracket;
            let subroutine = compile_subroutine(u, c, body, end_location)?;
            let subroutine = define_constant(c, subroutine)?;
            init_phasers.push(subroutine);
            Ok(())
        },

        Definition::Subroutine{name, body, right_curly_bracket, ..} => {
            let end_location = *right_curly_bracket;
            let index = globals[name].0 as usize;
            c[index] = compile_subroutine(u, c, body, end_location)?;
            Ok(())
        },

    }
}

/// Compile a sequence of statements to a subroutine value.
fn compile_subroutine(
    u: Weak<Unit>,
    c: &mut Vec<Value>,
    statements: &[Statement],
    end_location: Location,
) -> Result<Value>
{
    let procedure = compile_procedure(u, c, statements, end_location)?;
    Ok(Value::subroutine_from_procedure(Arc::new(procedure)))
}

/// Compile a sequence of statements to a procedure.
fn compile_procedure(
    u: Weak<Unit>,
    c: &mut Vec<Value>,
    statements: &[Statement],
    end_location: Location,
) -> Result<Verified>
{
    let mut b = Builder::new();

    // Allocate register for implicit result.
    with_register(&mut b, |b, result| {

        // Compile statements.
        for statement in statements {
            compile_statement(c, b, result, statement)?;
        }

        // If there were no statements, return undef.
        if statements.is_empty() {
            b.set_location(end_location);
            b.build(I::LoadUndef{target: result});
        }

        // Generate implicit return.
        b.build(I::Return{result});

        Ok(())

    })?;

    // Link the procedure.
    let procedure = b.link(u);

    // Return the procedure.
    // SAFETY: We do not generate bad procedures.
    Ok(unsafe { Verified::new_unchecked(procedure) })
}

/// Compile a statement.
fn compile_statement(
    c: &mut Vec<Value>,
    b: &mut Builder,
    target: Register,
    statement: &Statement,
) -> Result<()>
{
    match statement {

        Statement::Expression{expression, ..} =>
            compile_expression(c, b, target, expression),

    }
}

/// Compile an expression.
fn compile_expression(
    c: &mut Vec<Value>,
    b: &mut Builder,
    target: Register,
    expression: &Expression,
) -> Result<()>
{
    match expression {

        Expression::NumericAdd{..} =>
            todo!(),

        Expression::StringConcatenate{left, tilde, right} => {
            compile_expression(c, b, target, left)?;
            with_register(b, |b, tmp| {
                compile_expression(c, b, tmp, right)?;
                b.set_location(*tilde);
                b.build(I::StringConcatenate{target, left: target, right: tmp});
                Ok(())
            })
        },

        Expression::StringLiteral{value, location} => {
            let value = Value::string_from_bytes(value.clone())?;
            let constant = define_constant(c, value)?;
            b.set_location(*location);
            b.build(I::LoadConstant{target, constant});
            Ok(())
        },

    }
}

/* -------------------------------------------------------------------------- */
/*                                  Utilities                                 */
/* -------------------------------------------------------------------------- */

/// Define a new constant and return its index.
fn define_constant(constants: &mut Vec<Value>, value: Value)
    -> Result<Constant>
{
    let index = constants.len().try_into()
        .map_err(|_| Error::TooManyConstants)?;
    constants.push(value);
    Ok(Constant(index))
}

/// Less polymorphic form of [`Builder::with_register`].
fn with_register<F, R>(b: &mut Builder, f: F) -> Result<R>
    where F: FnOnce(&mut Builder, Register) -> Result<R>
{
    b.with_register(f)
}
