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

fn main()
{
	utils::set_internal_panic_hook();

	let args = args::parse();
	let file = utils::read_file(&args.input_files[0]);
	let tokens = lexer::tokenize(&args.input_files[0], &file);
	let ast = parser::parse(&args.input_files[0], &file, &tokens);

	parser::print_ast(&ast);

	let _ = args.output_file;
}
