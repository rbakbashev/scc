use std::collections::HashMap;

use crate::parser::{AST, Type};
use crate::utils::{CheckError, error};

#[rustfmt::skip]
pub enum Node
{
	FunctionDefinition { name: String, params: Vec<u32>, body: Vec<Node> },
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
		match node {
			Node::FunctionDefinition { name, params, body } => {
				println!("func {name} {params:?}");
				print(body);
			}
		}
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

	ir.push(Node::FunctionDefinition { name, params, body });
}

fn walk_function_body(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	expect(ast, Type::CompoundStatement);
}
