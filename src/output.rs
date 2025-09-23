#![allow(clippy::cast_sign_loss, unused_must_use)]

use std::collections::HashMap;
use std::fmt::Write;

use crate::args::ARGS;
use crate::codegen::{Assignment, Instruction};
use crate::utils::{CheckError, error};

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

	write_asm_epilogue(&mut out);

	out.into_bytes()
}

fn write_asm_prologue(out: &mut String)
{
	writeln!(out, "section .text");
	writeln!(out);
}

fn write_asm_instr(instr: &Instruction, out: &mut String)
{
	match instr {
		Instruction::FuncPrologue { name, stack_used } => {
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
		Instruction::Add { dst, src } => {
			writeln!(out, "\tadd {dst}, {src}");
		}
		Instruction::Sub { dst, src } => {
			writeln!(out, "\tsub {dst}, {src}");
		}
		Instruction::Return => {
			writeln!(out, "\tleave");
			writeln!(out, "\tret");
			writeln!(out);
		}
		Instruction::FuncCall { name } => {
			writeln!(out, "\tcall {name}");
		}
	}
}

fn write_asm_epilogue(out: &mut String)
{
	writeln!(out, "global _start");
	writeln!(out, "_start:");
	writeln!(out, "\tcall main");
	writeln!(out, "\tmov rdi, rax");
	writeln!(out, "\tmov rax, 60");
	writeln!(out, "\tsyscall");
}

fn construct_executable(code: &[Instruction]) -> Vec<u8>
{
	let (text, _entrypoint) = construct_code(code);

	text
}

fn construct_code(code: &[Instruction]) -> (Vec<u8>, usize)
{
	let mut out = Vec::new();
	let mut addresses = HashMap::new();
	let entrypoint;

	for instr in code {
		write_code_instr(instr, &mut addresses, &mut out);
	}

	entrypoint = out.len();

	write_code_epilogue(&addresses, &mut out);

	(out, entrypoint)
}

fn write_code_instr(instr: &Instruction, addresses: &mut HashMap<String, usize>, out: &mut Vec<u8>)
{
	let address;

	match instr {
		Instruction::FuncPrologue { name, stack_used } => {
			addresses.insert(name.clone(), out.len());
			write_fn_prologue(*stack_used, out);
		}
		Instruction::Move { to, from } => write_move(*to, *from, out),
		Instruction::MoveImm { dst, value } => write_move_imm(*dst, *value, out),
		Instruction::Add { dst, src } => write_add(*dst, *src, out),
		Instruction::Sub { dst, src } => write_sub(*dst, *src, out),
		Instruction::Return => write_return(out),
		Instruction::FuncCall { name } => {
			address = addresses.get(name).try_to(format!("find function {name:?}"));
			write_fn_call(*address, out);
		}
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
		(src, Assignment::Stack(offset)) => {
			out.push(0x8b);
			modrm(offset, src, out);
		}
		_ => error(format!("unexpected move case: {to:?} and {from:?}")),
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
		Assignment::ESI => 0b110,
		Assignment::EDI => 0b111,
		otherwise => error(format!("reg: unexpected stack assignment {otherwise:?}")),
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

fn write_add(dst: Assignment, src: Assignment, out: &mut Vec<u8>)
{
	match (dst, src) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("additions from memory to memory are invalid"),
		(Assignment::Stack(offset), src) => {
			out.push(0x01);
			modrm(offset, src, out);
		}
		_ => error(format!("unexpected addition case: {dst:?} += {src:?}")),
	}
}

fn write_sub(dst: Assignment, src: Assignment, out: &mut Vec<u8>)
{
	match (dst, src) {
		(Assignment::Stack(_), Assignment::Stack(_)) =>
			error("subtractions from memory to memory are invalid"),
		(Assignment::Stack(offset), src) => {
			out.push(0x29);
			modrm(offset, src, out);
		}
		_ => error(format!("unexpected subtraction case: {dst:?} += {src:?}")),
	}
}

fn write_return(out: &mut Vec<u8>)
{
	out.push(0xc9); // leave
	out.push(0xc3); // ret
}

fn write_fn_call(address: usize, out: &mut Vec<u8>)
{
	let idx = i32::try_from(out.len()).or_err("code len overflows u32");
	let address = i32::try_from(address).or_err("function's address overflows u32");
	let offset = idx - address;

	out.push(0xe8);
	out.extend(offset.to_le_bytes());
}

fn write_code_epilogue(addresses: &HashMap<String, usize>, out: &mut Vec<u8>)
{
	let main = addresses.get("main").try_to("find main function");

	write_fn_call(*main, out);

	out.extend([0x48, 0x89, 0xc7]); // mov rdi, rax

	// mov rax, 60
	out.push(0xb8);
	out.extend(zero_extend(60, 4));

	out.extend([0x0f, 0x05]); // syscall
}
