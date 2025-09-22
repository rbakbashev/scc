use std::collections::HashMap;

use crate::parser::{AST, Type};
use crate::utils::{CheckError, error};

#[rustfmt::skip]
pub enum Node
{
	FuncDef { name: String, params: Vec<u32>, body: Vec<Node> },
	Add { x: u32, y: u32, ret: u32 },
	Sub { x: u32, y: u32, ret: u32 },
	FuncCall { name: String, args: Vec<u32>, ret: u32 },
	Constant { value: i32, place: u32 },
	Return { place: u32 },
}

struct Scope
{
	stack: Vec<HashMap<String, u32>>,
	next: u32,
}

pub fn lower(ast: &AST) -> Vec<Node>
{
	let mut nodes = Vec::new();
	let mut scope = scope_new();

	walk(ast, &mut nodes, &mut scope);

	nodes
}

pub fn print(ir: &[Node])
{
	for node in ir {
		print_node(node);
	}
}

fn print_node(node: &Node)
{
	match node {
		Node::FuncDef { name, params, body } => {
			println!("FNDEF {name} {params:?}");
			print(body);
		}
		Node::Add { x, y, ret } => println!("ADD {x} {y} -> {ret}"),
		Node::Sub { x, y, ret } => println!("SUB {x} {y} -> {ret}"),
		Node::FuncCall { name, args, ret } => println!("FNCALL {name} {args:?} -> {ret}"),
		Node::Constant { value, place } => println!("CONST {value} -> {place}"),
		Node::Return { place } => println!("RET {place}"),
	}
}

fn expect(ast: &AST, expected: Type)
{
	let current = ast.ty;

	if current != expected {
		error(format!("unexpected AST node type: {current:?} != {expected:?}"));
	}
}

fn scope_new() -> Scope
{
	Scope { stack: vec![HashMap::new()], next: 0 }
}

fn scope_push(scope: &mut Scope)
{
	scope.stack.push(HashMap::new());
}

fn scope_pop(scope: &mut Scope)
{
	let num_vars = match scope.stack.last() {
		Some(last) => last.len(),
		None => return,
	};
	let num_u32 = u32::try_from(num_vars).or_err("number of places overflows u32");

	scope.stack.pop();

	scope.next -= num_u32;
}

fn scope_insert(scope: &mut Scope, name: String) -> u32
{
	let place;

	if let Some(place) = scope_lookup(scope, &name) {
		return place;
	}

	place = scope.next;

	scope.next += 1;
	scope.stack.last_mut().or_err("empty scope stack").insert(name, place);

	place
}

fn scope_lookup(scope: &Scope, name: &str) -> Option<u32>
{
	for level in scope.stack.iter().rev() {
		if let Some(&place) = level.get(name) {
			return Some(place);
		}
	}

	None
}

fn scope_allocate(scope: &mut Scope) -> u32
{
	let place = scope.next;

	scope.next += 1;

	place
}

fn walk(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	expect(ast, Type::TranslationUnit);

	for node in &ast.next {
		expect(node, Type::ExternalDeclaration);

		walk_function_def(&node.next[0], ir, scope);
	}
}

fn walk_function_def(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let decl;
	let name;
	let mut param_ident;
	let mut params = Vec::new();
	let mut body = Vec::new();

	expect(ast, Type::FunctionDefinition);

	decl = &ast.next[1].next[0];

	expect(decl, Type::FunctionDeclarator);

	name = decl.next[0].data.clone().or_err("function declarator has no name");

	scope_push(scope);

	for param in &decl.next[1].next {
		expect(param, Type::ParameterDeclaration);

		param_ident =
			param.next[1].next[0].data.clone().or_err("function parameter has no name");

		params.push(scope_insert(scope, param_ident));
	}

	walk_function_body(&ast.next[2], &mut body, scope);

	scope_pop(scope);

	ir.push(Node::FuncDef { name, params, body });
}

fn walk_function_body(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	expect(ast, Type::CompoundStatement);

	for block in &ast.next {
		expect(block, Type::BlockItem);

		walk_unlabeled_statement(&block.next[0], ir, scope);
	}
}

fn walk_unlabeled_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	expect(ast, Type::UnlabeledStatement);

	for next in &ast.next {
		match next.ty {
			Type::JumpStatement => walk_jump_statement(next, ir, scope),
			otherwise => error(format!("unexpected statement type: {otherwise:?}")),
		}
	}
}

fn walk_jump_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let place;

	place = walk_expression(&ast.next[0], ir, scope);

	ir.push(Node::Return { place });
}

fn walk_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::Expression);

	walk_assignment_expression(&ast.next[0], ir, scope)
}

fn walk_assignment_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::AssignmentExpression);

	walk_conditional_expression(&ast.next[0], ir, scope)
}

fn walk_conditional_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::ConditionalExpression);

	walk_logical_or_expression(&ast.next[0], ir, scope)
}

fn walk_logical_or_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::LogicalOrExpression);

	walk_logical_and_expression(&ast.next[0], ir, scope)
}

fn walk_logical_and_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::LogicalAndExpression);

	walk_inclusive_or_expression(&ast.next[0], ir, scope)
}

fn walk_inclusive_or_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::InclusiveOrExpression);

	walk_exclusive_or_expression(&ast.next[0], ir, scope)
}

fn walk_exclusive_or_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::ExclusiveOrExpression);

	walk_and_expression(&ast.next[0], ir, scope)
}

fn walk_and_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::AndExpression);

	walk_equality_expression(&ast.next[0], ir, scope)
}

fn walk_equality_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::EqualityExpression);

	walk_relational_expression(&ast.next[0], ir, scope)
}

fn walk_relational_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::RelationalExpression);

	walk_shift_expression(&ast.next[0], ir, scope)
}

fn walk_shift_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::ShiftExpression);

	walk_additive_expression(&ast.next[0], ir, scope)
}

fn walk_additive_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	let ret;
	let x;
	let y;
	let node;

	expect(ast, Type::AdditiveExpression);

	x = walk_multiplicative_expression(&ast.next[0], ir, scope);

	if ast.next.len() == 1 {
		return x;
	}

	y = walk_multiplicative_expression(&ast.next[1], ir, scope);

	ret = scope_allocate(scope);

	node = match ast.data.as_deref() {
		Some("+") => Node::Add { x, y, ret },
		Some("-") => Node::Sub { x, y, ret },
		Some(otherwise) => error(format!("unexpected additive data: {otherwise:?}")),
		None => error("additive expression's data not set"),
	};

	ir.push(node);

	ret
}

fn walk_multiplicative_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::MultiplicativeExpression);

	walk_cast_expression(&ast.next[0], ir, scope)
}

fn walk_cast_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::CastExpression);

	walk_unary_expression(&ast.next[0], ir, scope)
}

fn walk_unary_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	expect(ast, Type::UnaryExpression);

	walk_postfix_expression(&ast.next[0], ir, scope)
}

fn walk_postfix_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	let primary = &ast.next[0];
	let name;
	let mut args = Vec::new();
	let ret;

	expect(ast, Type::PostfixExpression);

	if ast.data.is_none() {
		return walk_primary_expression(primary, ir, scope);
	}

	assert_eq!(primary.next[0].ty, Type::Identifier);

	name = primary.next[0].data.clone().or_err("function name not set");

	for arg in &ast.next[1].next {
		args.push(walk_assignment_expression(arg, ir, scope));
	}

	ret = scope_allocate(scope);

	ir.push(Node::FuncCall { name, args, ret });

	ret
}

fn walk_primary_expression(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope) -> u32
{
	let next = &ast.next[0];
	let data = next.data.as_ref().or_err("primary expression's data not set");
	let value;
	let place;

	expect(ast, Type::PrimaryExpression);

	match next.ty {
		Type::Identifier => scope_lookup(scope, data)
			.or_err(format!("variable {data:?} was not found in the current scope")),
		Type::Constant => {
			value = data.parse::<i32>().try_to(format!("parse {data:?} as a number"));
			place = scope_allocate(scope);

			ir.push(Node::Constant { value, place });

			place
		}
		otherwise => error(format!("unexpected primary expression type {otherwise:?}")),
	}
}
