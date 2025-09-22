#![allow(unused_must_use)]

use std::fmt::Write;

use crate::args::ARGS;
use crate::codegen::Instruction;

pub fn construct_file(code: &[Instruction]) -> Vec<u8>
{
	assert!(ARGS.assembly);

	construct_assembly(code)
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
