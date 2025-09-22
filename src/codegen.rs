use crate::ir;
use crate::utils::error;

#[derive(Debug)]
pub enum Instruction
{
	FuncPrologue(String),
}

pub enum Register
{
	EAX,
	EBX,
	ECX,
	EDX,
	ESI,
	EDI,
	EBP,
	ESP,
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
		gen_func_def(name, params, body, inst);
	}
	else {
		ty = ir::format_node_type(node);
		error(format!("unexpected top-level IR node: {ty}"));
	}
}

fn gen_func_def(name: &str, params: &[u32], body: &[ir::Node], inst: &mut Vec<Instruction>)
{
	inst.push(Instruction::FuncPrologue(name.to_owned()));
}
