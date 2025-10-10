#![allow(
	clippy::derive_partial_eq_without_eq,
	clippy::enum_glob_use,
	clippy::enum_variant_names,
	clippy::format_push_string,
	clippy::match_same_arms,
	clippy::missing_const_for_fn,
	clippy::needless_late_init,
	clippy::needless_range_loop,
	clippy::option_if_let_else,
	clippy::struct_excessive_bools,
	clippy::struct_field_names,
	clippy::unnecessary_wraps,
	clippy::upper_case_acronyms
)]

mod args;
mod codegen;
mod elf;
mod ir;
mod lexer;
mod optparse;
mod output;
mod parser;
mod utils;

use args::ARGS;
use codegen::Instruction;
use utils::{is_source_file, warn};

fn main()
{
	utils::set_internal_panic_hook();
	args::parse();

	if ARGS.assembly {
		generate_assembly_files();
		return;
	}

	if ARGS.compile_only {
		generate_object_files();
		return;
	}

	generate_executable_file();
}

fn generate_assembly_files()
{
	let mut path;
	let mut instrs;
	let mut asm;

	for filename in &ARGS.input_files {
		if !is_source_file(filename) {
			warn(format!("input file {filename:?} ignored for generating assembly"));
			continue;
		}

		path = args::output_fname_for_indiv_files(&ARGS, filename);
		instrs = compile(filename);
		asm = output::construct_assembly(&instrs);

		utils::write_to_file(&path, &asm, false);
	}
}

fn generate_object_files()
{
	let mut path;
	let mut instrs;
	let mut code;
	let mut obj;

	for filename in &ARGS.input_files {
		if !is_source_file(filename) {
			warn(format!("input file {filename:?} ignored: not producing executable"));
			continue;
		}

		path = args::output_fname_for_indiv_files(&ARGS, filename);
		instrs = compile(filename);
		code = output::construct_code(&instrs);
		obj = elf::construct_object_file(code);

		utils::write_to_file(&path, &obj, false);
	}
}

fn generate_executable_file()
{
	let path = args::output_fname_for_single_output(&ARGS);
	let mut instrs;
	let mut code;
	let mut inputs = Vec::new();
	let exec;

	for filename in &ARGS.input_files {
		if !is_source_file(filename) {
			todo!();
		}

		instrs = compile(filename);
		code = output::construct_code(&instrs);

		inputs.push(code);
	}

	if ARGS.add_start_stub {
		inputs.push(output::construct_start_stub());
	}

	exec = elf::construct_executable(&inputs);

	utils::write_to_file(&path, &exec, true);
}

fn compile(filename: &str) -> Vec<Instruction>
{
	let file = utils::read_file(filename);
	let tokens = lexer::tokenize(filename, &file);
	let ast = parser::parse(filename, &file, &tokens);
	let ir = ir::lower(&ast);

	codegen::gen_instructions(&ir)
}
