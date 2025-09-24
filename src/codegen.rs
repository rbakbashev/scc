use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::args::ARGS;
use crate::ir::{Node, format_node_type};
use crate::utils::error;

#[derive(Debug)]
#[rustfmt::skip]
pub enum Instruction
{
	FuncPrologue { name: String, stack_used: i32 },
	Move { to: Assignment, from: Assignment },
	MoveImm { dst: Assignment, value: i32 },
	Add { dst: Assignment, src: Assignment },
	Sub { dst: Assignment, src: Assignment },
	Return,
	FuncCall { name: String },
}

#[derive(Clone, Copy, Debug)]
pub enum Assignment
{
	EAX,
	_EBX,
	ECX,
	EDX,
	ESI,
	EDI,
	_EBP,
	_ESP,
	Stack(i32),
}

use Assignment::*;

struct PlaceMap
{
	hmap: HashMap<u32, Assignment>,
	stack_used: i32,
}

static CALL_CONV: &[Assignment] = &[EDI, ESI, EDX, ECX];

impl Display for Assignment
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		if let Stack(offset) = self {
			return write!(f, "[rbp{offset:+}]");
		}

		write!(f, "{}", format!("{self:?}").to_lowercase())
	}
}

fn placemap_new() -> PlaceMap
{
	PlaceMap { hmap: HashMap::new(), stack_used: 0 }
}

fn placemap_assign_args(map: &mut PlaceMap, params: &[u32])
{
	let assignments = assign_args(params.len());

	for (&param, assignment) in params.iter().zip(assignments) {
		map.hmap.insert(param, assignment);
	}
}

fn assign_args(num_args: usize) -> Vec<Assignment>
{
	let mut out = Vec::new();
	let mut offset = 2;

	for i in 0..num_args {
		if i < CALL_CONV.len() {
			out.push(CALL_CONV[i]);
		}
		else {
			out.push(Stack(offset * 8));
			offset += 1;
		}
	}

	out
}

fn placemap_get(map: &mut PlaceMap, place: u32) -> Assignment
{
	let assignment;

	if let Some(assignment) = map.hmap.get(&place) {
		*assignment
	}
	else {
		map.stack_used += 8;
		assignment = Stack(-map.stack_used);

		map.hmap.insert(place, assignment);

		assignment
	}
}

pub fn gen_instructions(ir: &[Node]) -> Vec<Instruction>
{
	let mut out = Vec::new();

	for node in ir {
		gen_toplevel(node, &mut out);
	}

	if ARGS.verbose {
		println!();
		println!("{out:#?}");
	}

	out
}

fn gen_toplevel(node: &Node, inst: &mut Vec<Instruction>)
{
	if let Node::FuncDef { name, params, body } = node {
		gen_fn_def(name, params, body, inst);
	}
	else {
		error(format!("unexpected top-level IR node: {}", format_node_type(node)));
	}
}

fn gen_fn_def(name: &str, params: &[u32], body: &[Node], inst: &mut Vec<Instruction>)
{
	let name = name.to_string();
	let mut map = placemap_new();
	let mut fn_inst = Vec::new();

	placemap_assign_args(&mut map, params);

	for node in body {
		gen_node(node, &mut map, &mut fn_inst);
	}

	inst.push(Instruction::FuncPrologue { name, stack_used: map.stack_used });
	inst.append(&mut fn_inst);
}

fn gen_node(node: &Node, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	match node {
		Node::FuncDef { .. } => error("nested functions are not supported"),
		Node::Add { x, y, ret } => gen_add(*x, *y, *ret, map, inst),
		Node::Sub { x, y, ret } => gen_sub(*x, *y, *ret, map, inst),
		Node::FuncCall { name, args, ret } => gen_fn_call(name, args, *ret, map, inst),
		Node::Constant { value, place } => gen_const(*value, *place, map, inst),
		Node::Return { place } => gen_return(*place, map, inst),
		_ => todo!(),
	}
}

fn gen_add(x: u32, y: u32, ret: u32, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	let x = placemap_get(map, x);
	let y = placemap_get(map, y);
	let ret = placemap_get(map, ret);

	inst.push(Instruction::Move { to: ret, from: x });
	inst.push(Instruction::Add { dst: ret, src: y });
}

fn gen_sub(x: u32, y: u32, ret: u32, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	let x = placemap_get(map, x);
	let y = placemap_get(map, y);
	let ret = placemap_get(map, ret);

	inst.push(Instruction::Move { to: ret, from: x });
	inst.push(Instruction::Sub { dst: ret, src: y });
}

fn gen_const(value: i32, place: u32, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	let dst = placemap_get(map, place);

	inst.push(Instruction::MoveImm { dst, value });
}

fn gen_return(place: u32, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	let place = placemap_get(map, place);

	inst.push(Instruction::Move { to: EAX, from: place });
	inst.push(Instruction::Return);
}

fn gen_fn_call(name: &str, args: &[u32], ret: u32, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	let assignments = assign_args(args.len());
	let mut place_assignment;
	let ret = placemap_get(map, ret);
	let name = name.to_string();

	for (&place, assignment) in args.iter().zip(assignments) {
		place_assignment = placemap_get(map, place);

		inst.push(Instruction::Move { to: assignment, from: place_assignment });
	}

	inst.push(Instruction::FuncCall { name });
	inst.push(Instruction::Move { to: ret, from: EAX });
}
