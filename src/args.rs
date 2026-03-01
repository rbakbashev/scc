use std::ops::Deref;
use std::path::Path;
use std::sync::OnceLock;

use crate::optparse::{Args, Opt, arg_present, arg_values, collect_args, usage, version};
use crate::utils::{CheckError, error, format_list, warn};

pub struct ParsedArgs
{
	pub input_files: Vec<String>,
	pub output_file: Option<String>,
	pub verbose: bool,
	pub assembly: bool,
	pub compile_only: bool,
	pub add_start_stub: bool,
	pub wasm: bool,
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
		Opt::Flag { short: 's', long: "start", desc: "add _start stub that calls main" },
		Opt::Flag {
			short: 'w',
			long: "wasm",
			desc: "output WebAssembly (can be used with -S)",
		},
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

	let output_file = output_filename(args);
	let verbose = arg_present(args, 'V');
	let assembly = arg_present(args, 'S');
	let compile_only = arg_present(args, 'c');
	let add_start_stub = arg_present(args, 's');
	let wasm = arg_present(args, 'w');

	if input_files.len() > 1 && output_file.is_some() && (assembly || compile_only) {
		warn("ignoring provided output filename: multiple input files with -c or -S used");
	}

	if assembly && compile_only {
		warn("both -c and -S provided: doing only compilation regardless");
	}

	ParsedArgs {
		input_files,
		output_file,
		verbose,
		assembly,
		compile_only,
		add_start_stub,
		wasm,
	}
}

fn output_filename(args: &Args) -> Option<String>
{
	let outputs = arg_values(args, 'o');

	match outputs.as_slice() {
		[] => None,
		[one] => Some(one.clone()),
		many => error(format!("multiple output filenames provided: {}", format_list(many))),
	}
}

pub fn output_fname_for_indiv_files(parsed: &ParsedArgs, input_file: &str) -> String
{
	let extension = get_output_extension(parsed);

	if parsed.input_files.len() == 1
		&& let Some(provided_output_file) = &parsed.output_file
	{
		return provided_output_file.clone();
	}

	construct_output_filename(input_file, extension)
}

pub fn output_fname_for_single_output(parsed: &ParsedArgs) -> String
{
	if parsed.input_files.len() == 1 {
		return construct_output_filename(&parsed.input_files[0], "");
	}

	if let Some(provided_output_file) = &parsed.output_file {
		return provided_output_file.clone();
	}

	error("no output filename provided for multiple input files");
}

fn get_output_extension(parsed: &ParsedArgs) -> &'static str
{
	if parsed.wasm {
		if parsed.assembly {
			return "wat";
		}

		if parsed.compile_only {
			return "o";
		}

		"wasm"
	}
	else {
		if parsed.assembly {
			return "s";
		}

		if parsed.compile_only {
			return "o";
		}

		""
	}
}

fn construct_output_filename(input_file: &str, extension: &str) -> String
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
