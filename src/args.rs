use std::ops::Deref;
use std::path::Path;
use std::sync::OnceLock;

use crate::optparse::{Args, Opt, arg_present, arg_values, collect_args, usage, version};
use crate::utils::{CheckError, error, format_list, warn};

pub struct ParsedArgs
{
	pub input_files: Vec<String>,
	pub output_file: String,
	pub verbose: bool,
}

pub struct WriteOnce<T>
{
	data: OnceLock<T>,
}

pub static ARGS: WriteOnce<ParsedArgs> = writeonce_empty();

impl<T> Deref for WriteOnce<T>
{
	type Target = T;

	fn deref(&self) -> &T
	{
		self.data.get().or_err("writeonce cell unset")
	}
}

pub fn parse()
{
	let options = [
		Opt::Flag { short: 'h', long: "help", desc: "show this message" },
		Opt::Flag { short: 'v', long: "version", desc: "show version" },
		Opt::Value {
			short: 'o',
			long: "output",
			desc: "write output to <filename>",
			hint: "<filename>",
		},
		Opt::Flag { short: 'V', long: "verbose", desc: "enable verbose output" },
	];

	let results = collect_args(&options);

	if arg_present(&results, 'v') {
		version("scc");
	}

	if arg_present(&results, 'h') {
		usage("scc", "<input file(s)>", &options, 0);
	}

	writeonce_assign(&ARGS, into_parsed_args(&results));
}

fn into_parsed_args(args: &Args) -> ParsedArgs
{
	let input_files = if args.free.is_empty() {
		vec!["examples/add.c".to_owned()]
		// error("no input filename(s) given");
	}
	else {
		args.free.clone()
	};

	let output_file = get_output_filename(args, &input_files);

	let verbose = arg_present(args, 'V');

	ParsedArgs { input_files, output_file, verbose }
}

fn get_output_filename(args: &Args, input_files: &[String]) -> String
{
	let outputs = arg_values(args, 'o');

	match outputs.as_slice() {
		[] => construct_output_filename(input_files),
		[one] => one.clone(),
		many => error(format!("multiple output filenames provided: {}", format_list(many))),
	}
}

fn construct_output_filename(input_files: &[String]) -> String
{
	let input_file = match input_files {
		[one] => one,
		_many => error("no output filename provided"),
	};

	let basename = Path::new(input_file).file_name().or_err("input filename ends in '/..'");
	let mut path = Path::new(basename).to_path_buf();

	path.set_extension("");

	path.to_string_lossy().to_string()
}

const fn writeonce_empty<T>() -> WriteOnce<T>
{
	WriteOnce { data: OnceLock::new() }
}

fn writeonce_assign<T>(cell: &WriteOnce<T>, data: T)
{
	if cell.data.set(data).is_err() {
		warn("writeonce cell was already set");
	}
}
