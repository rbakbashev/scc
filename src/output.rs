#![allow(clippy::cast_sign_loss, unused_must_use)]

use std::collections::HashMap;
use std::fmt::Write;

use crate::args::ARGS;
use crate::codegen::{Assignment, Cond, Instruction};
use crate::elf::construct_elf;
use crate::ir::{ArithOp, format_arith};
use crate::utils::{CheckError, error};

struct Addresses
{
	map: HashMap<String, usize>,
	relocations: Vec<Relocation>,
}

struct Relocation
{
	label: u16,
	idx: usize,
	offset: usize,
	instr_len: i32,
}

fn addresses_get_label(addresses: &Addresses, label: u16) -> Option<usize>
{
	addresses.map.get(&format!("L{label}")).copied()
}

fn addresses_push_reloc(addr: &mut Addresses, label: u16, out: &[u8], offset: usize, instr_len: i32)
{
	addr.relocations.push(Relocation { label, idx: out.len(), offset, instr_len });
}

pub fn construct_file(code: &[Instruction]) -> Vec<u8>
{
	if ARGS.assembly {
		return construct_assembly(code);
	}

	construct_executable(code)
}

fn construct_assembly(code: &[Instruction]) -> Vec<u8>
{
	let mut out = String::new();

	write_asm_prologue(&mut out);

	for instr in code {
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

fn construct_executable(code: &[Instruction]) -> Vec<u8>
{
	let (text, entrypoint) = construct_code(code);

	construct_elf(text, entrypoint)
}

fn construct_code(code: &[Instruction]) -> (Vec<u8>, usize)
{
	let mut out = Vec::new();
	let mut addresses = Addresses { map: HashMap::new(), relocations: Vec::new() };
	let entrypoint;

	for instr in code {
		write_code_instr(instr, &mut addresses, &mut out);
	}

	entrypoint = out.len();

	write_code_epilogue(&addresses, &mut out);

	write_relocations(&mut out, &addresses);

	(out, entrypoint)
}

fn write_code_instr(instr: &Instruction, addresses: &mut Addresses, out: &mut Vec<u8>)
{
	let address;

	match instr {
		Instruction::FuncPrologue { name, stack_used } => {
			addresses.map.insert(name.clone(), out.len());
			write_fn_prologue(*stack_used, out);
		}
		Instruction::Move { to, from } => write_move(*to, *from, out),
		Instruction::MoveImm { dst, value } => write_move_imm(*dst, *value, out),
		Instruction::Arith { op, dst, src } => write_arith(*op, *dst, *src, out),
		Instruction::Return => write_return(out),
		Instruction::FuncCall { name } => {
			address = addresses.map.get(name).try_to(format!("find function {name:?}"));
			write_fn_call(*address, out);
		}
		Instruction::JumpCond { cond, label } => {
			write_jump_cond(*cond, *label, addresses, out);
		}
		Instruction::Jump { label } => write_jump(*label, addresses, out),
		Instruction::Label { name } => {
			addresses.map.insert(format!("L{name}"), out.len());
		}
		Instruction::Compare { x, y } => write_compare(*x, *y, out),
		Instruction::CompareImm { x, value } => write_compare_imm(*x, *value, out),
	}
}

fn write_fn_prologue(stack_used: i32, out: &mut Vec<u8>)
{
	out.extend([0x55]); // push rbp
	out.extend([0x48, 0x89, 0xe5]); // mov rbp, rsp

	// sub rsp, {stack_used}
	if let Ok(byte) = i8::try_from(stack_used) {
		out.extend([0x48, 0x83, 0xec]);
		out.push(byte as u8);
	}
	else {
		out.extend([0x48, 0x81, 0xec]);
		out.extend(zero_extend(stack_used, 5));
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

fn write_move(to: Assignment, from: Assignment, out: &mut Vec<u8>)
{
	match (to, from) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("moves from memory to memory are invalid"),
		(Assignment::Stack(offset), src) => {
			out.push(0x89);
			modrm(offset, src, out);
		}
		(dst, Assignment::Stack(offset)) => {
			out.push(0x8b);
			modrm(offset, dst, out);
		}
		(dst, src) => {
			out.push(0x89);
			modrm_regs(dst, src, out);
		}
	}
}

fn modrm(offset: i32, src: Assignment, out: &mut Vec<u8>)
{
	if let Ok(byte) = i8::try_from(offset) {
		push_modrm(0b01, reg_field(src), 0b101, out); // [rbp+disp8]
		out.push(byte as u8);
	}
	else {
		push_modrm(0b10, reg_field(src), 0b101, out); // [rbp+disp32]
		out.extend(offset.to_le_bytes());
	}
}

fn modrm_regs(x: Assignment, y: Assignment, out: &mut Vec<u8>)
{
	push_modrm(0b11, reg_field(y), reg_field(x), out);
}

fn modrm_single(offset: i32, digit: u8, out: &mut Vec<u8>)
{
	if let Ok(byte) = i8::try_from(offset) {
		push_modrm(0b01, digit, 0b101, out); // [rbp+disp8]
		out.push(byte as u8);
	}
	else {
		push_modrm(0b10, digit, 0b101, out); // [rbp+disp32]
		out.extend(offset.to_le_bytes());
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

fn push_modrm(modb: u8, reg: u8, rm: u8, out: &mut Vec<u8>)
{
	out.push((modb << 6) | (reg << 3) | rm);
}

fn write_move_imm(dst: Assignment, value: i32, out: &mut Vec<u8>)
{
	match dst {
		Assignment::Stack(offset) => {
			out.push(0xc7);
			modrm_single(offset, 0, out);
			out.extend(value.to_le_bytes());
		}
		_ => error(format!("unexpected move imm case {dst:?}")),
	}
}

fn write_arith(op: ArithOp, dst: Assignment, src: Assignment, out: &mut Vec<u8>)
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

fn write_arith_stack_reg(op: ArithOp, offset: i32, src: Assignment, out: &mut Vec<u8>)
{
	match op {
		ArithOp::Add => out.push(0x01),
		ArithOp::Sub => out.push(0x29),
	}

	modrm(offset, src, out);
}

fn write_arith_reg_stack(op: ArithOp, src: Assignment, offset: i32, out: &mut Vec<u8>)
{
	match op {
		ArithOp::Add => out.push(0x03),
		ArithOp::Sub => out.push(0x2b),
	}

	modrm(offset, src, out);
}

fn write_return(out: &mut Vec<u8>)
{
	out.push(0xc9); // leave
	out.push(0xc3); // ret
}

fn write_fn_call(address: usize, out: &mut Vec<u8>)
{
	let offset = rip_offset(address, out, 5);

	out.push(0xe8); // call
	out.extend(offset.to_le_bytes());
}

fn rip_offset(address: usize, out: &[u8], instr_len: i32) -> i32
{
	let idx = i32::try_from(out.len()).or_err("code len overflows u32");
	let address = i32::try_from(address).or_err("address overflows u32");

	address - idx - instr_len
}

fn write_jump_cond(cond: Cond, label: u16, addresses: &mut Addresses, out: &mut Vec<u8>)
{
	let offset;

	offset = if let Some(address) = addresses_get_label(addresses, label) {
		rip_offset(address, out, 6)
	}
	else {
		addresses_push_reloc(addresses, label, out, 2, 6);
		0
	};

	out.push(0x0f);

	match cond {
		Cond::LT => out.push(0x8c),
		Cond::GT => out.push(0x8f),
		Cond::LTE => out.push(0x8e),
		Cond::GTE => out.push(0x8d),
		Cond::NotEqual => out.push(0x85),
		Cond::Equal => out.push(0x84),
	}

	out.extend(offset.to_le_bytes());
}

fn write_jump(label: u16, addresses: &mut Addresses, out: &mut Vec<u8>)
{
	let offset;

	offset = if let Some(address) = addresses_get_label(addresses, label) {
		rip_offset(address, out, 5)
	}
	else {
		addresses_push_reloc(addresses, label, out, 1, 5);
		0
	};

	out.push(0xe9); // jmp

	out.extend(offset.to_le_bytes());
}

fn write_compare(x: Assignment, y: Assignment, out: &mut Vec<u8>)
{
	match (x, y) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("comparisons between two memory locations are invalid"),
		(src, Assignment::Stack(offset)) => {
			out.push(0x3b);
			modrm(offset, src, out);
		}
		_ => error(format!("unexpected comparison case: {x:?} {y:?}")),
	}
}

fn write_compare_imm(x: Assignment, value: i32, out: &mut Vec<u8>)
{
	if let Assignment::Stack(offset) = x {
		out.push(0x81);
		modrm_single(offset, 7, out);
		out.extend(value.to_le_bytes());
		return;
	}

	error(format!("unexpected case of comparison with immediate: {x:?}"));
}

fn write_code_epilogue(addresses: &Addresses, out: &mut Vec<u8>)
{
	let main = addresses.map.get("main").try_to("find main function");

	write_fn_call(*main, out);

	out.extend([0x48, 0x89, 0xc7]); // mov rdi, rax

	// mov rax, 60
	out.push(0xb8);
	out.extend(zero_extend(60, 4));

	out.extend([0x0f, 0x05]); // syscall
}

fn write_relocations(out: &mut [u8], addr: &Addresses)
{
	let mut label_str;
	let mut address;
	let mut addr_i32;
	let mut idx;
	let mut offset;
	let mut location;

	for relocation in &addr.relocations {
		label_str = format!("L{}", relocation.label);
		address = addr.map.get(&label_str).try_to(format!("find label {label_str}"));

		location = relocation.idx + relocation.offset;

		idx = i32::try_from(relocation.idx).or_err("code location overflows u32");
		addr_i32 = i32::try_from(*address).or_err("address overflows u32");

		offset = addr_i32 - idx - relocation.instr_len;

		out[location..location + 4].copy_from_slice(&offset.to_le_bytes());
	}
}
