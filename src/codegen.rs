use std::collections::HashMap;
use std::fmt::{self, Display};

use crate::args::ARGS;
use crate::ir::{Cmp, Node, format_node_type};
use crate::utils::{CheckError, error};

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
	TestForOne { x: Assignment },
	JumpCond { cond: Cond, label: u16 },
	Jump { label: u16 },
	Label { name: u16 },
	Compare { x: Assignment, y: Assignment, },
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

#[derive(Debug)]
pub enum Cond
{
	LT,
	GT,
	LTE,
	GTE,
	NonZero,
}

struct PlaceMap
{
	hmap: HashMap<u32, Assignment>,
	stack_used: i32,
}

struct State
{
	map: PlaceMap,
	next_label: u16,
	loop_exits: Vec<u16>,
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

fn comparison_to_condition(op: Cmp) -> Cond
{
	match op {
		Cmp::LT => Cond::LT,
		Cmp::GT => Cond::GT,
		Cmp::LTE => Cond::LTE,
		Cmp::GTE => Cond::GTE,
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

fn state_new() -> State
{
	State { map: placemap_new(), next_label: 0, loop_exits: Vec::new() }
}

fn assign_place(st: &mut State, place: u32) -> Assignment
{
	placemap_get(&mut st.map, place)
}

fn state_alloc_label(st: &mut State) -> u16
{
	let label = st.next_label;

	st.next_label += 1;

	label
}

pub fn gen_instructions(ir: &[Node]) -> Vec<Instruction>
{
	let mut out = Vec::new();

	for node in ir {
		gen_toplevel(node, &mut out);
	}

	if ARGS.verbose {
		println!();

		for inst in &out {
			println!("{inst:?}");
		}
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
	let mut state = state_new();
	let mut fn_inst = Vec::new();

	placemap_assign_args(&mut state.map, params);

	for node in body {
		gen_node(node, &mut state, &mut fn_inst);
	}

	inst.push(Instruction::FuncPrologue { name, stack_used: state.map.stack_used });
	inst.append(&mut fn_inst);
}

fn gen_node(node: &Node, st: &mut State, inst: &mut Vec<Instruction>)
{
	match node {
		Node::FuncDef { .. } => error("nested functions are not supported"),
		Node::Add { x, y, ret } => gen_add(*x, *y, *ret, st, inst),
		Node::Sub { x, y, ret } => gen_sub(*x, *y, *ret, st, inst),
		Node::FuncCall { name, args, ret } => gen_fn_call(name, args, *ret, st, inst),
		Node::Constant { value, place } => gen_const(*value, *place, st, inst),
		Node::Return { place } => gen_return(*place, st, inst),
		Node::If { cond, body } => gen_if(*cond, body, st, inst),
		Node::Loop { body } => gen_loop(body, st, inst),
		Node::Break => gen_break(st, inst),
		Node::Assign { lhs, rhs } => gen_assign(*lhs, *rhs, st, inst),
		Node::Compare { op, x, y, ret } => gen_compare(*op, *x, *y, *ret, st, inst),
	}
}

fn gen_add(x: u32, y: u32, ret: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let x = assign_place(st, x);
	let y = assign_place(st, y);
	let ret = assign_place(st, ret);

	inst.push(Instruction::Move { to: ret, from: x });
	inst.push(Instruction::Add { dst: ret, src: y });
}

fn gen_sub(x: u32, y: u32, ret: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let x = assign_place(st, x);
	let y = assign_place(st, y);
	let ret = assign_place(st, ret);

	inst.push(Instruction::Move { to: ret, from: x });
	inst.push(Instruction::Sub { dst: ret, src: y });
}

fn gen_const(value: i32, place: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let dst = assign_place(st, place);

	inst.push(Instruction::MoveImm { dst, value });
}

fn gen_return(place: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let place = assign_place(st, place);

	inst.push(Instruction::Move { to: EAX, from: place });
	inst.push(Instruction::Return);
}

fn gen_fn_call(name: &str, args: &[u32], ret: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let assignments = assign_args(args.len());
	let mut place_assignment;
	let ret = assign_place(st, ret);
	let name = name.to_string();

	for (&place, assignment) in args.iter().zip(assignments) {
		place_assignment = assign_place(st, place);

		inst.push(Instruction::Move { to: assignment, from: place_assignment });
	}

	inst.push(Instruction::FuncCall { name });
	inst.push(Instruction::Move { to: ret, from: EAX });
}

fn gen_if(cond: u32, body: &[Node], st: &mut State, inst: &mut Vec<Instruction>)
{
	let cond = assign_place(st, cond);
	let lbl_out = state_alloc_label(st);
	let mut body_inst = Vec::new();

	inst.push(Instruction::Move { to: EAX, from: cond });
	inst.push(Instruction::TestForOne { x: EAX });
	inst.push(Instruction::JumpCond { cond: Cond::NonZero, label: lbl_out });

	for node in body {
		gen_node(node, st, &mut body_inst);
	}

	inst.append(&mut body_inst);

	inst.push(Instruction::Label { name: lbl_out });
}

fn gen_loop(body: &[Node], st: &mut State, inst: &mut Vec<Instruction>)
{
	let lbl_start = state_alloc_label(st);
	let lbl_out = state_alloc_label(st);

	st.loop_exits.push(lbl_out);

	inst.push(Instruction::Label { name: lbl_start });

	for node in body {
		gen_node(node, st, inst);
	}

	inst.push(Instruction::Jump { label: lbl_start });
	inst.push(Instruction::Label { name: lbl_out });

	st.loop_exits.pop();
}

fn gen_break(st: &State, inst: &mut Vec<Instruction>)
{
	let label = *st.loop_exits.last().or_err("break outside of a loop");

	inst.push(Instruction::Jump { label });
}

fn gen_assign(lhs: u32, rhs: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let lhs = assign_place(st, lhs);
	let rhs = assign_place(st, rhs);

	inst.push(Instruction::Move { to: lhs, from: rhs });
}

fn gen_compare(op: Cmp, x: u32, y: u32, ret: u32, st: &mut State, inst: &mut Vec<Instruction>)
{
	let x = assign_place(st, x);
	let y = assign_place(st, y);
	let ret = assign_place(st, ret);
	let cond = comparison_to_condition(op);
	let lbl_true = state_alloc_label(st);
	let lbl_cont = state_alloc_label(st);

	inst.push(Instruction::Compare { x, y });
	inst.push(Instruction::JumpCond { cond, label: lbl_true });
	inst.push(Instruction::MoveImm { dst: ret, value: 0 });
	inst.push(Instruction::Jump { label: lbl_cont });
	inst.push(Instruction::Label { name: lbl_true });
	inst.push(Instruction::MoveImm { dst: ret, value: 1 });
	inst.push(Instruction::Label { name: lbl_cont });
}
