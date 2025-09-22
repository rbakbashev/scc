use std::collections::HashMap;

use crate::ir;
use crate::utils::error;

#[derive(Debug)]
#[rustfmt::skip]
pub enum Instruction
{
	FuncPrologue(String),
	FuncEpilogue,
	Move { to: PlaceAssignment, from: PlaceAssignment },
	MoveImm { dst: PlaceAssignment, value: i32 },
	Add { dst: PlaceAssignment, src: PlaceAssignment },
	Sub { dst: PlaceAssignment, src: PlaceAssignment },
	Return,
	FuncCall(String),
}

#[derive(Clone, Copy, Debug)]
pub enum PlaceAssignment
{
	EAX,
	EBX,
	ECX,
	EDX,
	ESI,
	EDI,
	EBP,
	ESP,
	Stack(i32),
}

use PlaceAssignment::*;

struct PlaceMap
{
	hmap: HashMap<u32, PlaceAssignment>,
	stack_ptr: i32,
}

static CALL_CONV: &[PlaceAssignment] = &[EDI, ESI, EDX, ECX];

fn placemap_new() -> PlaceMap
{
	PlaceMap { hmap: HashMap::new(), stack_ptr: 0 }
}

fn placemap_assign_args(map: &mut PlaceMap, params: &[u32])
{
	let assignments = assign_args(params.len());

	for (&param, assignment) in params.iter().zip(assignments) {
		map.hmap.insert(param, assignment);
	}
}

fn assign_args(num_args: usize) -> Vec<PlaceAssignment>
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

fn placemap_get(map: &mut PlaceMap, place: u32) -> PlaceAssignment
{
	let assignment;

	if let Some(assignment) = map.hmap.get(&place) {
		*assignment
	}
	else {
		map.stack_ptr -= 1;
		assignment = Stack(map.stack_ptr * 8);

		map.hmap.insert(place, assignment);

		assignment
	}
}

pub fn gen_instr(ir: &[ir::Node]) -> Vec<Instruction>
{
	let mut out = Vec::new();

	for node in ir {
		gen_toplevel(node, &mut out);
	}

	out
}

fn gen_toplevel(node: &ir::Node, inst: &mut Vec<Instruction>)
{
	let ty;

	if let ir::Node::FuncDef { name, params, body } = node {
		gen_fn_def(name, params, body, inst);
	}
	else {
		ty = ir::format_node_type(node);
		error(format!("unexpected top-level IR node: {ty}"));
	}
}

fn gen_fn_def(name: &str, params: &[u32], body: &[ir::Node], inst: &mut Vec<Instruction>)
{
	let mut map = placemap_new();

	placemap_assign_args(&mut map, params);

	inst.push(Instruction::FuncPrologue(name.to_owned()));

	for node in body {
		gen_node(node, &mut map, inst);
	}

	inst.push(Instruction::FuncEpilogue);
}

fn gen_node(node: &ir::Node, map: &mut PlaceMap, inst: &mut Vec<Instruction>)
{
	match node {
		ir::Node::FuncDef { .. } => error("nested functions are not supported"),
		ir::Node::Add { x, y, ret } => gen_add(*x, *y, *ret, map, inst),
		ir::Node::Sub { x, y, ret } => gen_sub(*x, *y, *ret, map, inst),
		ir::Node::FuncCall { name, args, ret } => gen_fn_call(name, args, *ret, map, inst),
		ir::Node::Constant { value, place } => gen_const(*value, *place, map, inst),
		ir::Node::Return { place } => gen_return(*place, map, inst),
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
	let assignemnts = assign_args(args.len());
	let mut place_assignment;
	let ret = placemap_get(map, ret);

	for (&place, assignment) in args.iter().zip(assignemnts) {
		place_assignment = placemap_get(map, place);

		inst.push(Instruction::Move { to: assignment, from: place_assignment });
	}

	inst.push(Instruction::FuncCall(name.to_string()));
	inst.push(Instruction::Move { to: ret, from: EAX });
}
