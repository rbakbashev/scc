#![allow(clippy::match_same_arms, clippy::missing_const_for_fn, clippy::option_if_let_else)]

mod args;
mod optparse;
mod utils;

fn main()
{
	utils::set_internal_panic_hook();

	let args = args::parse();

	println!("input_files={:?}, output_file={}", args.input_files, args.output_file);
}
