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

fn main()
{
	utils::set_internal_panic_hook();
	args::parse();

	let filename = &ARGS.input_files[0];
	let file = utils::read_file(filename);
	let tokens = lexer::tokenize(filename, &file);
	let ast = parser::parse(filename, &file, &tokens);
	let ir = ir::lower(&ast);
	let instrs = codegen::gen_instructions(&ir);
	let output = output::construct_file(&instrs);

	utils::write_to_file(&ARGS.output_file, &output, !ARGS.assembly);
}
