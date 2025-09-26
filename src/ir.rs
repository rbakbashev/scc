use std::collections::HashMap;

use crate::args::ARGS;
use crate::parser::{AST, Type};
use crate::utils::{CheckError, error};

#[rustfmt::skip]
pub enum Node
{
	FuncDef { name: String, params: Vec<u32>, body: Vec<Node> },
	Arith { op: ArithOp, x: u32, y: u32, ret: u32 },
	FuncCall { name: String, args: Vec<u32>, ret: u32 },
	Constant { value: i32, place: u32 },
	Return { place: u32 },
	If { cond: u32, body: Vec<Node> },
	IfNot { cond: u32, body: Vec<Node> },
	Loop { body: Vec<Node> },
	Break,
	Assign { lhs: u32, rhs: u32 },
	Compare { op: Cmp, x: u32, y: u32, ret: u32 },
}

#[derive(Clone, Copy, Debug)]
pub enum ArithOp
{
	Add,
	Sub,
}

#[derive(Clone, Copy, Debug)]
pub enum Cmp
{
	LT,
	GT,
	LTE,
	GTE,
}

struct Scope
{
	stack: Vec<ScopeLevel>,
	next: u32,
}

struct ScopeLevel
{
	map: HashMap<String, u32>,
	allocated: usize,
}

pub fn lower(ast: &AST) -> Vec<Node>
{
	let mut nodes = Vec::new();
	let mut scope = scope_new();

	walk(ast, &mut nodes, &mut scope);

	if ARGS.verbose {
		println!();
		print(&nodes);
	}

	nodes
}

pub fn print(ir: &[Node])
{
	print_nodes(ir, 0);
}

fn print_nodes(ir: &[Node], level: usize)
{
	for node in ir {
		print_node(node, level);
	}
}

fn print_node(node: &Node, level: usize)
{
	let ty = format_node_type(node);
	let indent = " ".repeat(2 * level);

	print!("{indent}{ty} ");

	match node {
		Node::FuncDef { name, params, body } => {
			println!("{name} {}", format_places(params));
			print_nodes(body, level + 1);
		}
		Node::Arith { op, x, y, ret } => {
			println!("${x} {} ${y} -> ${ret}", format_arith(*op));
		}
		Node::FuncCall { name, args, ret } =>
			println!("{name} {} -> ${ret}", format_places(args)),
		Node::Constant { value, place } => println!("{value} -> ${place}"),
		Node::Return { place } => println!("${place}"),
		Node::If { cond, body } => {
			println!("${cond}");
			print_nodes(body, level + 1);
		}
		Node::IfNot { cond, body } => {
			println!("${cond}");
			print_nodes(body, level + 1);
		}
		Node::Loop { body } => {
			println!();
			print_nodes(body, level + 1);
		}
		Node::Break => println!(),
		Node::Assign { lhs, rhs } => println!("${lhs} = ${rhs}"),
		Node::Compare { op, x, y, ret } => println!("${x} {op:?} ${y} -> ${ret}"),
	}
}

pub fn format_node_type(node: &Node) -> &str
{
	match node {
		Node::FuncDef { .. } => "FNDEF",
		Node::Arith { .. } => "ARITH",
		Node::FuncCall { .. } => "FNCALL",
		Node::Constant { .. } => "CONST",
		Node::Return { .. } => "RET",
		Node::If { .. } => "IF",
		Node::IfNot { .. } => "IFNOT",
		Node::Loop { .. } => "LOOP",
		Node::Break => "BREAK",
		Node::Assign { .. } => "ASSIGN",
		Node::Compare { .. } => "CMP",
	}
}

fn format_places(places: &[u32]) -> String
{
	let mut out = "(".to_string();
	let mut iter = places.iter().peekable();

	while let Some(place) = iter.next() {
		out += &format!("${place}");

		if iter.peek().is_some() {
			out += ", ";
		}
	}

	out += ")";

	out
}

pub fn format_arith(op: ArithOp) -> &'static str
{
	match op {
		ArithOp::Add => "+",
		ArithOp::Sub => "-",
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
	Scope { stack: vec![ScopeLevel { map: HashMap::new(), allocated: 0 }], next: 0 }
}

fn scope_push(scope: &mut Scope)
{
	scope.stack.push(ScopeLevel { map: HashMap::new(), allocated: 0 });
}

fn scope_pop(scope: &mut Scope)
{
	let num_vars = match scope.stack.last() {
		Some(level) => level.allocated,
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

	scope_top(scope).map.insert(name, place);
	scope_top(scope).allocated += 1;

	place
}

fn scope_top(scope: &mut Scope) -> &mut ScopeLevel
{
	scope.stack.last_mut().or_err("empty scope stack")
}

fn scope_lookup(scope: &Scope, name: &str) -> Option<u32>
{
	for level in scope.stack.iter().rev() {
		if let Some(&place) = level.map.get(name) {
			return Some(place);
		}
	}

	None
}

fn scope_allocate(scope: &mut Scope) -> u32
{
	let place = scope.next;

	scope.next += 1;
	scope_top(scope).allocated += 1;

	place
}

fn scope_assign(scope: &mut Scope, name: String, place: u32)
{
	scope_top(scope).map.insert(name, place);
}

fn walk(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let mut next;

	expect(ast, Type::TranslationUnit);

	for node in &ast.next {
		expect(node, Type::ExternalDeclaration);

		next = &node.next[0];

		match next.ty {
			Type::FunctionDefinition => walk_function_def(next, ir, scope),
			Type::Declaration => walk_declaration(next, ir, scope),
			otherwise => error(format!("unexpected top-level item: {otherwise:?}")),
		}
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

	walk_compound_statement(&ast.next[2], &mut body, scope);

	scope_pop(scope);

	ir.push(Node::FuncDef { name, params, body });
}

fn walk_compound_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let mut next;

	expect(ast, Type::CompoundStatement);

	for block in &ast.next {
		expect(block, Type::BlockItem);

		next = &block.next[0];

		match next.ty {
			Type::Declaration => walk_declaration(next, ir, scope),
			Type::UnlabeledStatement => walk_unlabeled_statement(next, ir, scope),
			otherwise => error(format!("unexpected block item type: {otherwise:?}")),
		}
	}
}

fn walk_declaration(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let list = &ast.next[1];

	expect(list, Type::InitDeclaratorList);

	for init_decl in &list.next {
		walk_init_declarator(init_decl, ir, scope);
	}
}

fn walk_init_declarator(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let decl = &ast.next[0];
	let mut is_func = false;
	let name;
	let place;

	expect(ast, Type::InitDeclarator);

	name = match decl.next[0].ty {
		Type::FunctionDeclarator => {
			is_func = true;
			decl.next[0].next[0].data.clone().or_err("function declaration has no name")
		}
		Type::Identifier => decl.next[0].data.clone().or_err("declaration has no name"),
		otherwise => error(format!("unexpected declarator type {otherwise:?}")),
	};

	if ast.next.len() == 2 {
		if is_func {
			error("attempted to initialize a function with a value");
		}

		place = walk_assignment_expression(&ast.next[1].next[0], ir, scope);
		scope_assign(scope, name, place);
	}
	else {
		scope_insert(scope, name);
	}
}

fn walk_unlabeled_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	expect(ast, Type::UnlabeledStatement);

	for next in &ast.next {
		match next.ty {
			Type::ExpressionStatement => walk_expression_statement(next, ir, scope),
			Type::PrimaryBlock => walk_primary_block(next, ir, scope),
			Type::JumpStatement => walk_jump_statement(next, ir, scope),
			otherwise => error(format!("unexpected statement type: {otherwise:?}")),
		}
	}
}

fn walk_expression_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	for expr in &ast.next {
		expect(expr, Type::Expression);

		walk_expression(expr, ir, scope);
	}
}

fn walk_primary_block(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let next = &ast.next[0];

	match next.ty {
		Type::CompoundStatement => walk_compound_statement(next, ir, scope),
		Type::SelectionStatement => walk_selection_statement(next, ir, scope),
		Type::IterationStatement => walk_iteration_statement(next, ir, scope),
		otherwise => error(format!("unexpected primary block type: {otherwise:?}")),
	}
}

fn walk_selection_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let cond = walk_expression(&ast.next[0], ir, scope);
	let mut body = Vec::new();

	walk_unlabeled_statement(&ast.next[1], &mut body, scope);

	ir.push(Node::If { cond, body });
}

fn walk_iteration_statement(ast: &AST, ir: &mut Vec<Node>, scope: &mut Scope)
{
	let cond;
	let mut body = Vec::new();

	cond = walk_expression(&ast.next[0], &mut body, scope);

	body.push(Node::IfNot { cond, body: vec![Node::Break] });

	walk_unlabeled_statement(&ast.next[1], &mut body, scope);

	ir.push(Node::Loop { body });
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
	let lhs;
	let rhs;
	let node;

	expect(ast, Type::AssignmentExpression);

	if ast.next.len() == 1 {
		return walk_conditional_expression(&ast.next[0], ir, scope);
	}

	lhs = walk_unary_expression(&ast.next[0], ir, scope);
	rhs = walk_assignment_expression(&ast.next[1], ir, scope);

	node = match ast.data.as_deref() {
		Some("=") => Node::Assign { lhs, rhs },
		Some("-=") => Node::Arith { op: ArithOp::Sub, x: lhs, y: rhs, ret: lhs },
		Some(otherwise) => error(format!("unexpected assignment data: {otherwise:?}")),
		None => error("assignment expression data not set"),
	};

	ir.push(node);

	lhs
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
	let x;
	let y;
	let ret;
	let op;

	expect(ast, Type::RelationalExpression);

	x = walk_shift_expression(&ast.next[0], ir, scope);

	if ast.next.len() == 1 {
		return x;
	}

	y = walk_shift_expression(&ast.next[1], ir, scope);

	ret = scope_allocate(scope);

	op = match ast.data.as_deref() {
		Some("<") => Cmp::LT,
		Some(">") => Cmp::GT,
		Some("<=") => Cmp::LTE,
		Some(">=") => Cmp::GTE,
		Some(otherwise) => error(format!("unexpected relational data: {otherwise:?}")),
		None => error("relational expression's data not set"),
	};

	ir.push(Node::Compare { op, x, y, ret });

	ret
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
		Some("+") => Node::Arith { op: ArithOp::Add, x, y, ret },
		Some("-") => Node::Arith { op: ArithOp::Sub, x, y, ret },
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
