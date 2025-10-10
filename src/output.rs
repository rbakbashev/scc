#![allow(clippy::cast_sign_loss, unused_must_use)]

use std::collections::HashMap;
use std::fmt::Write;

use crate::args::ARGS;
use crate::codegen::{Assignment, Cond, Instruction};
use crate::elf::construct_elf;
use crate::ir::{ArithOp, format_arith};
use crate::utils::{CheckError, error};

struct Output
{
	addresses: HashMap<String, usize>,
	relocations: Vec<Relocation>,
	bytes: Vec<u8>,
	globals: Vec<(String, usize)>,
}

struct Relocation
{
	label: u16,
	idx: usize,
	offset: usize,
	instr_len: i32,
}

pub struct Code
{
	pub text: Vec<u8>,
	pub entrypoint: usize,
	pub globals: Vec<(String, usize)>,
}

fn empty_output() -> Output
{
	Output {
		addresses: HashMap::new(),
		relocations: Vec::new(),
		bytes: Vec::new(),
		globals: Vec::new(),
	}
}

fn push(out: &mut Output, data: impl IntoIterator<Item = u8>)
{
	out.bytes.extend(data);
}

fn push_byte(out: &mut Output, byte: u8)
{
	out.bytes.push(byte);
}

fn get_address(out: &Output, name: &str) -> Option<usize>
{
	out.addresses.get(name).copied()
}

fn get_label_address(out: &Output, label: u16) -> Option<usize>
{
	get_address(out, &format!("L{label}"))
}

fn add_relocation(out: &mut Output, label: u16, offset: usize, instr_len: i32)
{
	let idx = out.bytes.len();

	out.relocations.push(Relocation { label, idx, offset, instr_len });
}

pub fn construct_file(instrs: &[Instruction]) -> Vec<u8>
{
	if ARGS.assembly {
		return construct_assembly(instrs);
	}

	construct_binary(instrs)
}

fn construct_assembly(instrs: &[Instruction]) -> Vec<u8>
{
	let mut out = String::new();

	write_asm_prologue(&mut out);

	for instr in instrs {
		write_asm_instr(instr, &mut out);
	}

	if !ARGS.compile_only {
		write_asm_epilogue(&mut out);
	}

	out.into_bytes()
}

fn write_asm_prologue(out: &mut String)
{
	writeln!(out, "section .text");
}

fn write_asm_instr(instr: &Instruction, out: &mut String)
{
	match instr {
		Instruction::FuncPrologue { name, stack_used } => {
			writeln!(out);
			writeln!(out, "global {name}");
			writeln!(out, "{name}:");
			writeln!(out, "\tpush rbp");
			writeln!(out, "\tmov rbp, rsp");
			writeln!(out, "\tsub rsp, {stack_used}");
		}
		Instruction::Move { to, from } => {
			writeln!(out, "\tmov {to}, {from}");
		}
		Instruction::MoveImm { dst, value } => {
			writeln!(out, "\tmov dword {dst}, {value}");
		}
		Instruction::Arith { op, dst, src } => {
			writeln!(out, "\t{} {dst}, {src}", arith_instr(*op));
		}
		Instruction::Return => {
			writeln!(out, "\tleave");
			writeln!(out, "\tret");
		}
		Instruction::FuncCall { name } => {
			writeln!(out, "\tcall {name}");
		}
		Instruction::JumpCond { cond, label } => {
			writeln!(out, "\t{} .L{label}", cond_instr(*cond));
		}
		Instruction::Jump { label } => {
			writeln!(out, "\tjmp .L{label}");
		}
		Instruction::Label { name } => {
			writeln!(out, ".L{name}:");
		}
		Instruction::Compare { x, y } => {
			writeln!(out, "\tcmp {x}, {y}");
		}
		Instruction::CompareImm { x, value } => {
			writeln!(out, "\tcmp dword {x}, {value}");
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

fn cond_instr(cond: Cond) -> &'static str
{
	match cond {
		Cond::LT => "jl",
		Cond::GT => "jg",
		Cond::LTE => "jle",
		Cond::GTE => "jge",
		Cond::NotEqual => "jne",
		Cond::Equal => "je",
	}
}

fn write_asm_epilogue(out: &mut String)
{
	writeln!(out);
	writeln!(out, "global _start");
	writeln!(out, "_start:");
	writeln!(out, "\tcall main");
	writeln!(out, "\tmov rdi, rax");
	writeln!(out, "\tmov rax, 60");
	writeln!(out, "\tsyscall");
}

fn construct_binary(instrs: &[Instruction]) -> Vec<u8>
{
	let output = construct_code(instrs);

	construct_elf(output)
}

fn construct_code(instrs: &[Instruction]) -> Code
{
	let mut out = empty_output();
	let entrypoint;

	for instr in instrs {
		write_code_instr(instr, &mut out);
	}

	entrypoint = out.bytes.len();

	if !ARGS.compile_only {
		write_code_epilogue(&mut out);
	}

	write_relocations(&mut out);

	Code { text: out.bytes, entrypoint, globals: out.globals }
}

fn write_code_instr(instr: &Instruction, out: &mut Output)
{
	let address;

	match instr {
		Instruction::FuncPrologue { name, stack_used } => {
			out.addresses.insert(name.clone(), out.bytes.len());
			out.globals.push((name.clone(), out.bytes.len()));
			write_fn_prologue(*stack_used, out);
		}
		Instruction::Move { to, from } => write_move(*to, *from, out),
		Instruction::MoveImm { dst, value } => write_move_imm(*dst, *value, out),
		Instruction::Arith { op, dst, src } => write_arith(*op, *dst, *src, out),
		Instruction::Return => write_return(out),
		Instruction::FuncCall { name } => {
			address = get_address(out, name).try_to(format!("find function {name:?}"));
			write_fn_call(address, out);
		}
		Instruction::JumpCond { cond, label } => {
			write_jump_cond(*cond, *label, out);
		}
		Instruction::Jump { label } => write_jump(*label, out),
		Instruction::Label { name } => {
			out.addresses.insert(format!("L{name}"), out.bytes.len());
		}
		Instruction::Compare { x, y } => write_compare(*x, *y, out),
		Instruction::CompareImm { x, value } => write_compare_imm(*x, *value, out),
	}
}

fn write_fn_prologue(stack_used: i32, out: &mut Output)
{
	push(out, [0x55]); // push rbp
	push(out, [0x48, 0x89, 0xe5]); // mov rbp, rsp

	// sub rsp, {stack_used}
	if let Ok(byte) = i8::try_from(stack_used) {
		push(out, [0x48, 0x83, 0xec]);
		push_byte(out, byte as u8);
	}
	else {
		push(out, [0x48, 0x81, 0xec]);
		push(out, zero_extend(stack_used, 5));
	}
}

fn zero_extend(imm: i32, num_bytes: usize) -> Vec<u8>
{
	let mut out = imm.to_le_bytes().to_vec();

	while out.len() < num_bytes {
		out.push(0);
	}

	out
}

fn write_move(to: Assignment, from: Assignment, out: &mut Output)
{
	match (to, from) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("moves from memory to memory are invalid"),
		(Assignment::Stack(offset), src) => {
			push_byte(out, 0x89);
			modrm(offset, src, out);
		}
		(dst, Assignment::Stack(offset)) => {
			push_byte(out, 0x8b);
			modrm(offset, dst, out);
		}
		(dst, src) => {
			push_byte(out, 0x89);
			modrm_regs(dst, src, out);
		}
	}
}

fn modrm(offset: i32, src: Assignment, out: &mut Output)
{
	if let Ok(byte) = i8::try_from(offset) {
		push_modrm(0b01, reg_field(src), 0b101, out); // [rbp+disp8]
		push_byte(out, byte as u8);
	}
	else {
		push_modrm(0b10, reg_field(src), 0b101, out); // [rbp+disp32]
		push(out, offset.to_le_bytes());
	}
}

fn modrm_regs(x: Assignment, y: Assignment, out: &mut Output)
{
	push_modrm(0b11, reg_field(y), reg_field(x), out);
}

fn modrm_single(offset: i32, digit: u8, out: &mut Output)
{
	if let Ok(byte) = i8::try_from(offset) {
		push_modrm(0b01, digit, 0b101, out); // [rbp+disp8]
		push_byte(out, byte as u8);
	}
	else {
		push_modrm(0b10, digit, 0b101, out); // [rbp+disp32]
		push(out, offset.to_le_bytes());
	}
}

fn reg_field(register: Assignment) -> u8
{
	match register {
		Assignment::EAX => 0b000,
		Assignment::ECX => 0b001,
		Assignment::EDX => 0b010,
		Assignment::EBX => 0b011,
		Assignment::ESI => 0b110,
		Assignment::EDI => 0b111,
		Assignment::Stack(_) => error("unexpected stack assignment"),
	}
}

fn push_modrm(modb: u8, reg: u8, rm: u8, out: &mut Output)
{
	push_byte(out, (modb << 6) | (reg << 3) | rm);
}

fn write_move_imm(dst: Assignment, value: i32, out: &mut Output)
{
	match dst {
		Assignment::Stack(offset) => {
			push_byte(out, 0xc7);
			modrm_single(offset, 0, out);
			push(out, value.to_le_bytes());
		}
		_ => error(format!("unexpected move imm case {dst:?}")),
	}
}

fn write_arith(op: ArithOp, dst: Assignment, src: Assignment, out: &mut Output)
{
	let fmt;

	match (dst, src) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("arithmetic operations from memory to memory are invalid"),
		(Assignment::Stack(offset), src) => write_arith_stack_reg(op, offset, src, out),
		(dst, Assignment::Stack(offset)) => write_arith_reg_stack(op, dst, offset, out),
		_ => {
			fmt = format_arith(op);
			error(format!("unexpected arithmetic case: {dst:?} {fmt}= {src:?}"));
		}
	}
}

fn write_arith_stack_reg(op: ArithOp, offset: i32, src: Assignment, out: &mut Output)
{
	match op {
		ArithOp::Add => push_byte(out, 0x01),
		ArithOp::Sub => push_byte(out, 0x29),
	}

	modrm(offset, src, out);
}

fn write_arith_reg_stack(op: ArithOp, src: Assignment, offset: i32, out: &mut Output)
{
	match op {
		ArithOp::Add => push_byte(out, 0x03),
		ArithOp::Sub => push_byte(out, 0x2b),
	}

	modrm(offset, src, out);
}

fn write_return(out: &mut Output)
{
	push_byte(out, 0xc9); // leave
	push_byte(out, 0xc3); // ret
}

fn write_fn_call(address: usize, out: &mut Output)
{
	let offset = rip_offset(address, &out.bytes, 5);

	push_byte(out, 0xe8); // call
	push(out, offset.to_le_bytes());
}

fn rip_offset(address: usize, bytes: &[u8], instr_len: i32) -> i32
{
	let idx = i32::try_from(bytes.len()).or_err("code len overflows u32");
	let address = i32::try_from(address).or_err("address overflows u32");

	address - idx - instr_len
}

fn write_jump_cond(cond: Cond, label: u16, out: &mut Output)
{
	let offset;

	offset = if let Some(address) = get_label_address(out, label) {
		rip_offset(address, &out.bytes, 6)
	}
	else {
		add_relocation(out, label, 2, 6);
		0
	};

	push_byte(out, 0x0f);

	match cond {
		Cond::LT => push_byte(out, 0x8c),
		Cond::GT => push_byte(out, 0x8f),
		Cond::LTE => push_byte(out, 0x8e),
		Cond::GTE => push_byte(out, 0x8d),
		Cond::NotEqual => push_byte(out, 0x85),
		Cond::Equal => push_byte(out, 0x84),
	}

	push(out, offset.to_le_bytes());
}

fn write_jump(label: u16, out: &mut Output)
{
	let offset;

	offset = if let Some(address) = get_label_address(out, label) {
		rip_offset(address, &out.bytes, 5)
	}
	else {
		add_relocation(out, label, 1, 5);
		0
	};

	push_byte(out, 0xe9); // jmp

	push(out, offset.to_le_bytes());
}

fn write_compare(x: Assignment, y: Assignment, out: &mut Output)
{
	match (x, y) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("comparisons between two memory locations are invalid"),
		(src, Assignment::Stack(offset)) => {
			push_byte(out, 0x3b);
			modrm(offset, src, out);
		}
		_ => error(format!("unexpected comparison case: {x:?} {y:?}")),
	}
}

fn write_compare_imm(x: Assignment, value: i32, out: &mut Output)
{
	if let Assignment::Stack(offset) = x {
		push_byte(out, 0x81);
		modrm_single(offset, 7, out);
		push(out, value.to_le_bytes());
		return;
	}

	error(format!("unexpected case of comparison with immediate: {x:?}"));
}

fn write_code_epilogue(out: &mut Output)
{
	let main = get_address(out, "main").try_to("find main function");

	write_fn_call(main, out);

	push(out, [0x48, 0x89, 0xc7]); // mov rdi, rax

	// mov rax, 60
	push_byte(out, 0xb8);
	push(out, zero_extend(60, 4));

	push(out, [0x0f, 0x05]); // syscall
}

fn write_relocations(out: &mut Output)
{
	let mut label_str;
	let mut address;
	let mut addr_i32;
	let mut idx;
	let mut offset;
	let mut location;

	for relocation in &out.relocations {
		label_str = format!("L{}", relocation.label);
		address = get_address(out, &label_str).try_to(format!("find label {label_str}"));

		location = relocation.idx + relocation.offset;

		idx = i32::try_from(relocation.idx).or_err("code location overflows u32");
		addr_i32 = i32::try_from(address).or_err("address overflows u32");

		offset = addr_i32 - idx - relocation.instr_len;

		out.bytes[location..location + 4].copy_from_slice(&offset.to_le_bytes());
	}
}
