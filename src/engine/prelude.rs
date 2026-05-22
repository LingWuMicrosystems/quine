use quine_core::{common::Value, related_egraph::NativeFn, types::BaseType};

use crate::engine::EngineContext;

fn add(args: &[Value]) -> Value {
    Value(args[0].0.wrapping_add(args[1].0))
}
fn sub(args: &[Value]) -> Value {
    Value(args[0].0.wrapping_sub(args[1].0))
}
fn mul(args: &[Value]) -> Value {
    Value(args[0].0.wrapping_mul(args[1].0))
}
fn div(args: &[Value]) -> Value {
    Value(args[0].0 / args[1].0)
}

fn eq(args: &[Value]) -> Value {
    Value((args[0] == args[1]) as u64)
}
fn neq(args: &[Value]) -> Value {
    Value((args[0] != args[1]) as u64)
}
fn lt(args: &[Value]) -> Value {
    Value((args[0].0 < args[1].0) as u64)
}
fn gt(args: &[Value]) -> Value {
    Value((args[0].0 > args[1].0) as u64)
}
fn leq(args: &[Value]) -> Value {
    Value((args[0].0 <= args[1].0) as u64)
}
fn geq(args: &[Value]) -> Value {
    Value((args[0].0 >= args[1].0) as u64)
}

fn not(args: &[Value]) -> Value {
    Value((args[0].0 == 0) as u64)
}
fn and(args: &[Value]) -> Value {
    Value((args[0].0 != 0 && args[1].0 != 0) as u64)
}
fn or(args: &[Value]) -> Value {
    Value((args[0].0 != 0 || args[1].0 != 0) as u64)
}

pub fn register_prelude(ctx: &mut EngineContext) {
    let i32_2 = &[BaseType::I32, BaseType::I32];
    let bool_1 = &[BaseType::I1];
    let bool_2 = &[BaseType::I1, BaseType::I1];

    ctx.register_native("add", i32_2, BaseType::I32, add as NativeFn);
    ctx.register_native("sub", i32_2, BaseType::I32, sub as NativeFn);
    ctx.register_native("mul", i32_2, BaseType::I32, mul as NativeFn);
    ctx.register_native("div", i32_2, BaseType::I32, div as NativeFn);
    ctx.register_native("eq", i32_2, BaseType::I1, eq as NativeFn);
    ctx.register_native("neq", i32_2, BaseType::I1, neq as NativeFn);
    ctx.register_native("lt", i32_2, BaseType::I1, lt as NativeFn);
    ctx.register_native("gt", i32_2, BaseType::I1, gt as NativeFn);
    ctx.register_native("leq", i32_2, BaseType::I1, leq as NativeFn);
    ctx.register_native("geq", i32_2, BaseType::I1, geq as NativeFn);
    ctx.register_native("not", bool_1, BaseType::I1, not as NativeFn);
    ctx.register_native("and", bool_2, BaseType::I1, and as NativeFn);
    ctx.register_native("or", bool_2, BaseType::I1, or as NativeFn);
}
