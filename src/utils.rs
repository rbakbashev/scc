use std::panic::PanicHookInfo;

const RED: &str = "\x1b[31m";
const NORMAL: &str = "\x1b[m";

pub fn set_internal_panic_hook()
{
	std::panic::set_hook(Box::new(panic_hook_internal));
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
