use std::fmt::Display;
use std::fs::File;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::panic::PanicHookInfo;

pub trait CheckError<T>: Sized
{
	fn or_err(self, msg: impl Display) -> T;

	fn try_to(self, action: impl Display) -> T
	{
		self.or_err(format!("failed to {action}"))
	}
}

impl<T> CheckError<T> for Option<T>
{
	fn or_err(self, msg: impl Display) -> T
	{
		match self {
			Some(t) => t,
			None => error(msg),
		}
	}
}

impl<T, E: Display> CheckError<T> for Result<T, E>
{
	fn or_err(self, msg: impl Display) -> T
	{
		match self {
			Ok(t) => t,
			Err(err) => error(format!("{msg}: {err}")),
		}
	}
}

const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const NORMAL: &str = "\x1b[m";

pub fn set_internal_panic_hook()
{
	std::panic::set_hook(Box::new(panic_hook_internal));
}

fn set_user_panic_hook()
{
	std::panic::set_hook(Box::new(panic_hook_user));
}

fn panic_hook_internal(info: &PanicHookInfo)
{
	eprint!("{RED}internal compiler error{NORMAL}");

	if let Some(msg) = payload_as_str(info) {
		eprint!(": {msg}");
	}

	if let Some(loc) = info.location() {
		eprint!(" at {loc}");
	}

	eprintln!();
}

fn panic_hook_user(info: &PanicHookInfo)
{
	eprint!("{RED}error{NORMAL}");

	if let Some(msg) = payload_as_str(info) {
		eprint!(": {msg}");
	}

	eprintln!();
}

fn payload_as_str<'i>(info: &'i PanicHookInfo) -> Option<&'i str>
{
	if let Some(s) = info.payload().downcast_ref::<&str>() {
		return Some(s);
	}

	if let Some(s) = info.payload().downcast_ref::<String>() {
		return Some(s);
	}

	None
}

pub fn error(msg: impl Display) -> !
{
	set_user_panic_hook();

	panic!("{msg}");
}

pub fn warn(msg: impl Display)
{
	eprintln!("{YELLOW}warning{NORMAL}: {msg}");
}

pub fn intersperse<I: Iterator<Item = String>>(iter: I, separator: &str) -> String
{
	let mut out = String::new();
	let mut peekable = iter.peekable();

	while let Some(next) = peekable.next() {
		out += &next;

		if peekable.peek().is_some() {
			out += separator;
		}
	}

	out
}

pub fn format_list(items: &[impl Display]) -> String
{
	let mut out = String::new();

	if items.is_empty() {
		return "\"\"".to_string();
	}

	if items.len() == 1 {
		return format!("\"{}\"", items[0]);
	}

	for (i, item) in items.iter().enumerate() {
		if i == items.len() - 1 {
			out += " and ";
		}
		else if i != 0 {
			out += ", ";
		}

		out += &format!("\"{item}\"");
	}

	out
}

pub fn read_file(path: &str) -> String
{
	std::fs::read_to_string(path).try_to(format!("read file {path:?}"))
}

pub fn write_to_file(path: &str, contents: &[u8], executable: bool)
{
	let mode = if executable { 0o744 } else { 0o644 };

	let mut file = File::options()
		.create(true)
		.write(true)
		.truncate(true)
		.mode(mode)
		.open(path)
		.try_to(format!("create file {path:?}"));

	file.write_all(contents).try_to(format!("write to file {path:?}"));
}
