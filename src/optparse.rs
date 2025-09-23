use std::collections::HashMap;
use std::process::exit;

use crate::utils::{CheckError, error, intersperse};

#[rustfmt::skip]
pub enum Opt
{
	Flag { short: char, long: Str, desc: Str },
	Value { short: char, long: Str, desc: Str, hint: Str },
}

#[derive(Default)]
pub struct Args
{
	pub spec: HashMap<char, Vec<String>>,
	pub free: Vec<String>,
}

type Str = &'static str;

type ArgsIterator = std::env::Args;

pub fn collect_args(options: &[Opt]) -> Args
{
	let mut iter = std::env::args();
	let mut args = Args::default();

	let _progname = iter.next();

	while let Some(arg) = iter.next() {
		process_arg(arg, options, &mut iter, &mut args);
	}

	args
}

fn process_arg(arg: String, options: &[Opt], iter: &mut ArgsIterator, args: &mut Args)
{
	for option in options {
		if match_option(&arg, option, iter, args) {
			return;
		}
	}

	if arg.starts_with('-') {
		if matches_merged_options(&arg, options, args) {
			return;
		}

		error(format!("unrecognized option '{arg}'"));
	}

	args.free.push(arg);
}

fn match_option(arg: &str, option: &Opt, iter: &mut ArgsIterator, args: &mut Args) -> bool
{
	if !opt_matches(option, arg) {
		return false;
	}

	let short = opt_short(option);
	let entry = args.spec.entry(short).or_default();

	match option {
		Opt::Flag { .. } => {}
		Opt::Value { .. } => entry.push(get_next_arg(option, iter)),
	}

	true
}

fn matches_merged_options(arg: &str, options: &[Opt], args: &mut Args) -> bool
{
	if arg.starts_with("--") || arg.len() == 2 || arg == "-" {
		return false;
	}

	for ch in arg.chars().skip(1) {
		if !matches_short_opts(ch, options, args) {
			error(format!("unrecognized short option {ch:?} in merged option {arg:?}"));
		}
	}

	true
}

fn matches_short_opts(ch: char, options: &[Opt], args: &mut Args) -> bool
{
	let mut short;

	for option in options {
		short = opt_short(option);

		if short != ch {
			continue;
		}

		if matches!(option, Opt::Value { .. }) {
			error(format!("merged option {} requires an argument", opt_usage(option)));
		}

		args.spec.entry(short).or_default();

		return true;
	}

	false
}

fn opt_matches(option: &Opt, arg: &str) -> bool
{
	let short = format!("-{}", opt_short(option));
	let long = format!("--{}", opt_long(option));

	arg == short || arg == long
}

fn opt_short(option: &Opt) -> char
{
	match option {
		Opt::Flag { short, .. } => *short,
		Opt::Value { short, .. } => *short,
	}
}

fn opt_long(option: &Opt) -> &str
{
	match option {
		Opt::Flag { long, .. } => long,
		Opt::Value { long, .. } => long,
	}
}

fn opt_desc(option: &Opt) -> &str
{
	match option {
		Opt::Flag { desc, .. } => desc,
		Opt::Value { desc, .. } => desc,
	}
}

fn opt_usage(option: &Opt) -> String
{
	match option {
		Opt::Flag { short, long, .. } => format!("-{short}, --{long}"),
		Opt::Value { short, long, hint, .. } => format!("-{short}, --{long} {hint}"),
	}
}

fn opt_usage_short(option: &Opt) -> String
{
	match option {
		Opt::Flag { short, .. } => format!("[-{short}]"),
		Opt::Value { short, hint, .. } => format!("[-{short} {hint}]"),
	}
}

fn get_next_arg(option: &Opt, iter: &mut ArgsIterator) -> String
{
	iter.next().or_err(format!("missing argument for the [{}] option", opt_usage(option)))
}

pub fn arg_present(args: &Args, short: char) -> bool
{
	args.spec.contains_key(&short)
}

pub fn arg_values(args: &Args, short: char) -> Vec<String>
{
	args.spec.get(&short).unwrap_or(&vec![]).clone()
}

pub fn version(name: &str) -> !
{
	println!("{name} {}", option_env!("CARGO_PKG_VERSION").unwrap_or("unknown version"));

	exit(0);
}

pub fn usage(name: &str, quip: &str, options: &[Opt], exitcode: i32) -> !
{
	let short_opts = intersperse(options.iter().map(opt_usage_short), " ");
	let indent = "    ";

	println!("Usage: {name} {short_opts} {quip}");
	println!();

	for option in options {
		println!("{indent}{:<25} {}", opt_usage(option), opt_desc(option));
	}

	exit(exitcode);
}
