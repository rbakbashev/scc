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
	pub assembly: bool,
	pub compile_only: bool,
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
		Opt::Flag { short: 'S', long: "assembly", desc: "output assembly" },
		Opt::Flag { short: 'c', long: "compile-only", desc: "do not link" },
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
		error("no input filename(s) given");
	}
	else {
		args.free.clone()
	};

	let assembly = arg_present(args, 'S');
	let verbose = arg_present(args, 'V');
	let compile_only = arg_present(args, 'c');

	let extension = get_output_extension(assembly, compile_only);
	let output_file = get_output_filename(args, &input_files, extension);

	ParsedArgs { input_files, output_file, verbose, assembly, compile_only }
}

fn get_output_extension(assembly: bool, compile_only: bool) -> &'static str
{
	if assembly {
		return "s";
	}

	if compile_only {
		return "o";
	}

	""
}

fn get_output_filename(args: &Args, input_files: &[String], extension: &str) -> String
{
	let outputs = arg_values(args, 'o');

	match outputs.as_slice() {
		[] => input_files_to_output_filename(input_files, extension),
		[one] => one.clone(),
		many => error(format!("multiple output filenames provided: {}", format_list(many))),
	}
}

fn input_files_to_output_filename(input_files: &[String], extension: &str) -> String
{
	let input_file = match input_files {
		[one] => one,
		_many => error("no output filename provided"),
	};

	construct_output_filename(input_file, extension)
}

pub fn construct_output_filename(input_file: &str, extension: &str) -> String
{
	let basename = Path::new(input_file).file_name().or_err("input filename ends in '/..'");
	let mut path = Path::new(basename).to_path_buf();

	path.set_extension(extension);

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
