#![allow(
	clippy::enum_glob_use,
	clippy::enum_variant_names,
	clippy::format_push_string,
	clippy::match_same_arms,
	clippy::missing_const_for_fn,
	clippy::needless_late_init,
	clippy::option_if_let_else,
	clippy::unnecessary_wraps,
	clippy::upper_case_acronyms
)]

mod args;
mod lexer;
mod optparse;
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

	parser::print_ast(&ast);

	let _ = ARGS.output_file;
}
