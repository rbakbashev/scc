use std::fmt::Display;
use std::iter::Peekable;
use std::str::Chars;

use crate::utils::error;

#[derive(Debug)]
pub enum Token<'file>
{
	Identifier(&'file str),
	Punctuator(&'file str),
	Integer(i32),
}

struct FileReader<'file>
{
	filename: &'file str,
	input: &'file str,
	iter: Peekable<Chars<'file>>,
	pos: usize,
	line: u32,
	column: u32,
}

pub fn tokenize<'file>(filename: &'file str, input: &'file str) -> Vec<Token<'file>>
{
	let mut read = reader(filename, input);
	let mut out = Vec::new();

	while let Some(ch) = reader_curr(&mut read) {
		if is_whitespace(ch) {
			reader_consume(&mut read);
			continue;
		}

		if let Some(token) = eat_identifier(ch, &mut read) {
			out.push(token);
			continue;
		}

		if let Some(token) = eat_punctuator(ch, &mut read) {
			out.push(token);
			continue;
		}

		if let Some(token) = eat_integer(ch, &mut read) {
			out.push(token);
			continue;
		}

		reader_error(&read, format!("unhandled character: {ch:?}"));
	}

	out
}

fn reader<'file>(filename: &'file str, input: &'file str) -> FileReader<'file>
{
	FileReader { filename, input, iter: input.chars().peekable(), pos: 0, line: 1, column: 1 }
}

fn reader_curr(read: &mut FileReader) -> Option<char>
{
	read.iter.peek().copied()
}

fn reader_consume(read: &mut FileReader)
{
	let curr;

	match read.iter.next() {
		Some(ch) => curr = ch,
		None => return,
	}

	read.pos += 1;
	read.column += 1;

	if curr == '\n' {
		read.column = 1;
		read.line += 1;
	}
}

fn reader_error(read: &FileReader, msg: impl Display) -> !
{
	error(format!("{msg} at {}:{}:{}", read.filename, read.line, read.column));
}

fn is_whitespace(ch: char) -> bool
{
	matches!(ch, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}')
}

fn is_identifier_start(ch: char) -> bool
{
	matches!(ch, 'a' ..= 'z' | 'A' ..= 'Z' | '_')
}

fn is_identifier_continue(ch: char) -> bool
{
	matches!(ch, 'a' ..= 'z' | 'A' ..= 'Z' | '0' ..= '9' | '_')
}

#[rustfmt::skip]
fn is_punctuation(ch: char) -> bool
{
	matches!(ch, '!' | '%' | '&' | '(' | ')' | '*' | '+' | ',' | '-' | '.' | '/' | ':' | ';'
		| '<' | '=' | '>' | '?' | '[' | ']' | '^' | '{' | '|' | '}' | '~')
}

#[rustfmt::skip]
fn is_compound_punctuator(s: &str) -> bool
{
	matches!(s, "!=" | "%=" | "&&" | "&=" | "*=" | "++" | "+=" | "--" | "-=" | "->" | "..."
		| "/=" | "::" | "<<" | "<<=" | "<=" | "==" | ">=" | ">>" | ">>=" | "^=" | "|="
		| "||")
}

fn eat_identifier<'file>(curr: char, read: &mut FileReader<'file>) -> Option<Token<'file>>
{
	let start = read.pos;
	let value;

	if !is_identifier_start(curr) {
		return None;
	}

	reader_consume(read);

	while let Some(curr) = reader_curr(read) {
		if !is_identifier_continue(curr) {
			break;
		}

		reader_consume(read);
	}

	value = &read.input[start..read.pos];

	Some(Token::Identifier(value))
}

fn eat_punctuator<'file>(curr: char, read: &mut FileReader<'file>) -> Option<Token<'file>>
{
	let start = read.pos;
	let mut current = &read.input[start..=start];
	let mut new;

	if !is_punctuation(curr) {
		return None;
	}

	reader_consume(read);

	while reader_curr(read).is_some() {
		new = &read.input[start..=read.pos];

		if !is_compound_punctuator(new) {
			break;
		}

		current = new;

		reader_consume(read);
	}

	Some(Token::Punctuator(current))
}

fn eat_integer<'file>(curr: char, read: &mut FileReader<'file>) -> Option<Token<'file>>
{
	let mut value;

	if !curr.is_ascii_digit() {
		return None;
	}

	value = curr as u8 - b'0';

	reader_consume(read);

	while let Some(curr) = reader_curr(read) {
		if !curr.is_ascii_digit() {
			break;
		}

		value *= 10;
		value += curr as u8 - b'0';

		reader_consume(read);
	}

	Some(Token::Integer(value.into()))
}
