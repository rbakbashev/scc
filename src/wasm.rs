#![allow(unused_must_use)]

use std::fmt::Write;

use crate::args::ARGS;
use crate::ir::{ArithOp, Cmp, Node, format_node_type};
use crate::utils::error;

pub fn construct_text(ir: &[Node]) -> Vec<u8>
{
	let mut out = String::new();

	write_text_prologue(&mut out);
	write_text_instructions(ir, &mut out);
	write_text_epilogue(&mut out);

	out.into_bytes()
}

fn write_text_prologue(out: &mut String)
{
	writeln!(out, "(module");
	writeln!(out, "\t(import \"wasi_snapshot_preview1\" \"proc_exit\"");
	writeln!(out, "\t\t(func $proc_exit (param i32)))");
	writeln!(out, "\t(memory (export \"memory\") 1)");
	writeln!(out);
}

fn write_text_instructions(ir: &[Node], out: &mut String)
{
	for node in ir {
		if let Node::FuncDef { name, params, body } = node {
			write_fn_def(name, params, body, out);
		}
		else {
			error(format!("unexpected top-level IR node: {}", format_node_type(node)));
		}
	}
}

fn write_fn_def(name: &str, params: &[u32], body: &[Node], out: &mut String)
{
	let locals = num_locals(params, body);
	let mut labels = 0;

	writeln!(out, "\t(func ${name}");

	for _ in 0..params.len() {
		writeln!(out, "\t\t(param i32)");
	}

	writeln!(out, "\t\t(result i32)");

	for _ in 0..=locals {
		writeln!(out, "\t\t(local i32)");
	}

	write_nodes(body, &mut labels, out);

	writeln!(out, "\t)");
	writeln!(out);
}

fn num_locals(params: &[u32], body: &[Node]) -> usize
{
	let max_place = max_place(body) as usize;
	let num_params = params.len();

	assert!(num_params <= max_place);

	max_place - num_params
}

fn max_place(body: &[Node]) -> u32
{
	let mut max = 0;

	for node in body {
		match node {
			Node::FuncDef { .. } => error("nested functions are not supported"),
			Node::Arith { x, y, ret, .. } => {
				max = u32::max(max, *x);
				max = u32::max(max, *y);
				max = u32::max(max, *ret);
			}
			Node::FuncCall { ret, .. } => {
				max = u32::max(max, *ret);
			}
			Node::Constant { place, .. } => {
				max = u32::max(max, *place);
			}
			Node::Return { place } => {
				max = u32::max(max, *place);
			}
			Node::If { body, .. } => {
				max = u32::max(max, max_place(body));
			}
			Node::IfNot { body, .. } => {
				max = u32::max(max, max_place(body));
			}
			Node::Loop { body } => {
				max = u32::max(max, max_place(body));
			}
			Node::Break => {}
			Node::Assign { lhs, rhs } => {
				max = u32::max(max, *lhs);
				max = u32::max(max, *rhs);
			}
			Node::Compare { x, y, ret, .. } => {
				max = u32::max(max, *x);
				max = u32::max(max, *y);
				max = u32::max(max, *ret);
			}
		}
	}

	max
}

fn write_nodes(body: &[Node], labels: &mut u16, out: &mut String)
{
	for node in body {
		write_node(node, labels, out);
	}
}

fn write_node(node: &Node, labels: &mut u16, out: &mut String)
{
	let label;

	match node {
		Node::FuncDef { .. } => unreachable!(),
		Node::Arith { op, x, y, ret } => {
			writeln!(out, "\t\tlocal.get {x}");
			writeln!(out, "\t\tlocal.get {y}");
			writeln!(out, "\t\ti32.{}", arith_instr(*op));
			writeln!(out, "\t\tlocal.set {ret}");
		}
		Node::FuncCall { name, args, ret } => {
			load_places(args, out);
			writeln!(out, "\t\tcall ${name}");
			writeln!(out, "\t\tlocal.set {ret}");
		}
		Node::Constant { value, place } => {
			writeln!(out, "\t\ti32.const {value}");
			writeln!(out, "\t\tlocal.set {place}");
		}
		Node::Return { place } => {
			writeln!(out, "\t\tlocal.get {place}");
			writeln!(out, "\t\treturn");
		}
		Node::If { cond, body } => {
			writeln!(out, "\t\tlocal.get {cond}");
			writeln!(out, "\t\tif");
			write_nodes(body, labels, out);
			writeln!(out, "\t\tend");
		}
		Node::IfNot { cond, body } => {
			writeln!(out, "\t\tlocal.get {cond}");
			writeln!(out, "\t\ti32.eqz");
			writeln!(out, "\t\tif");
			write_nodes(body, labels, out);
			writeln!(out, "\t\tend");
		}
		Node::Loop { body } => {
			label = *labels;
			*labels += 1;

			writeln!(out, "\t\t(block $L{label}_break");
			writeln!(out, "\t\t(loop $L{label}_cont");
			write_nodes(body, labels, out);
			writeln!(out, "\t\tbr $L{label}_cont");
			writeln!(out, "\t\t)");
			writeln!(out, "\t\t)");

			*labels -= 1;
		}
		Node::Break => {
			label = *labels - 1;
			writeln!(out, "\t\tbr $L{label}_break");
		}
		Node::Assign { lhs, rhs } => {
			writeln!(out, "\t\tlocal.get {rhs}");
			writeln!(out, "\t\tlocal.set {lhs}");
		}
		Node::Compare { op, x, y, ret } => {
			writeln!(out, "\t\tlocal.get {x}");
			writeln!(out, "\t\tlocal.get {y}");
			writeln!(out, "\t\ti32.{}", cmp_instr(*op));
			writeln!(out, "\t\tlocal.set {ret}");
		}
	}
}

fn arith_instr(op: ArithOp) -> &'static str
{
	match op {
		ArithOp::Add => "add",
		ArithOp::Sub => "sub",
	}
}

fn cmp_instr(op: Cmp) -> &'static str
{
	match op {
		Cmp::LT => "lt_s",
		Cmp::GT => "gt_s",
		Cmp::LTE => "le_s",
		Cmp::GTE => "ge_s",
	}
}

fn load_places(places: &[u32], out: &mut String)
{
	for place in places {
		writeln!(out, "\t\tlocal.get {place}");
	}
}

fn write_text_epilogue(out: &mut String)
{
	if ARGS.add_start_stub {
		write_text_start_stub(out);
	}

	writeln!(out, ")");
}

fn write_text_start_stub(out: &mut String)
{
	writeln!(out, "\t(func $_start");
	writeln!(out, "\t\tcall $main");
	writeln!(out, "\t\tcall $proc_exit");
	writeln!(out, "\t)");
	writeln!(out);
	writeln!(out, "\t(start $_start)");
}
