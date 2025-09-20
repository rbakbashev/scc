use std::fmt::Display;
use std::iter::Peekable;
use std::str::Chars;

use crate::args::ARGS;
use crate::utils::{CheckError, error};

pub struct Token
{
	pub ty: TokenType,
	pub start: u32,
	pub end: u32,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum TokenType
{
	Keyword,
	Identifier,
	Punctuator,
	Integer,
	EOF,
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

pub fn tokenize(filename: &str, input: &str) -> Vec<Token>
{
	let mut read = reader(filename, input);
	let mut out = Vec::new();

	if input.is_empty() {
		error(format!("file {filename:?} is empty"));
	}

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

		lex_error(&read, format!("unhandled character: {ch:?}"));
	}

	out.push(eof_token(input));

	if ARGS.verbose {
		print_token_list(&out, input);
	}

	out
}

pub fn print_token_list(tokens: &[Token], input: &str)
{
	for token in tokens {
		let text = token_text(token, input);

		println!("{:?} {text:?}", token.ty);
	}
}

pub fn token_text<'file>(token: &Token, input: &'file str) -> &'file str
{
	let range = (token.start as usize)..=(token.end as usize);

	&input[range]
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

fn lex_error(read: &FileReader, msg: impl Display) -> !
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
fn is_keyword(s: &str) -> bool
{
	matches!(s, "bool" | "break" | "case" | "char" | "const" | "continue" | "default" | "do"
		| "double" | "else" | "enum" | "extern" | "false" | "float" | "for" | "goto" | "if"
		| "inline" | "int" | "long" | "nullptr" | "restrict" | "return" | "short" | "signed"
		| "sizeof" | "static" | "struct" | "switch" | "true" | "typedef" | "union"
		| "unsigned" | "void" | "volatile" | "while")
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

fn eat_identifier(curr: char, read: &mut FileReader) -> Option<Token>
{
	let start = read.pos;
	let ty;

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

	ty = if is_keyword(&read.input[start..read.pos]) {
		TokenType::Keyword
	}
	else {
		TokenType::Identifier
	};

	token(ty, start, read.pos - 1)
}

fn eat_punctuator(curr: char, read: &mut FileReader) -> Option<Token>
{
	let start = read.pos;
	let mut current_end = start;
	let mut new_end;

	if !is_punctuation(curr) {
		return None;
	}

	reader_consume(read);

	while reader_curr(read).is_some() {
		new_end = read.pos;

		if !is_compound_punctuator(&read.input[start..=new_end]) {
			break;
		}

		current_end = new_end;

		reader_consume(read);
	}

	token(TokenType::Punctuator, start, current_end)
}

fn eat_integer(curr: char, read: &mut FileReader) -> Option<Token>
{
	let start = read.pos;

	if !curr.is_ascii_digit() {
		return None;
	}

	reader_consume(read);

	while let Some(curr) = reader_curr(read) {
		if !curr.is_ascii_digit() {
			break;
		}

		reader_consume(read);
	}

	token(TokenType::Integer, start, read.pos - 1)
}

fn token(ty: TokenType, start: usize, end: usize) -> Option<Token>
{
	let start = start.try_into().or_err("start overflows u32");
	let end = end.try_into().or_err("end overflows u32");

	Some(Token { ty, start, end })
}

fn eof_token(input: &str) -> Token
{
	let last = (input.len() - 1).try_into().or_err("len overflows u32");

	Token { ty: TokenType::EOF, start: last, end: last }
}
