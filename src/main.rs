#![allow(
	clippy::format_push_string,
	clippy::match_same_arms,
	clippy::missing_const_for_fn,
	clippy::needless_late_init,
	clippy::option_if_let_else,
	clippy::unnecessary_wraps
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

	let _ = args.output_file;
}
