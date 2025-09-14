use std::iter::Peekable;
use std::str::Chars;

use crate::utils::error;

#[derive(Debug)]
pub enum Token
{
	Keyword(String),
	Identifier(String),
	Punctuator(String),
	Integer(i32),
}

pub fn tokenize(input: &str) -> Vec<Token>
{
	let mut iter = input.chars().peekable();
	let mut out = Vec::new();

	while let Some(ch) = iter.next() {
		if is_whitespace(ch) {
			continue;
		}

		if let Some(token) = eat_identifier(ch, &mut iter) {
			out.push(token);
			continue;
		}

		if let Some(token) = eat_punctuator(ch, &mut iter) {
			out.push(token);
			continue;
		}

		if let Some(token) = eat_integer(ch, &mut iter) {
			out.push(token);
			continue;
		}

		error(format!("unhandled character: {ch:?}"));
	}

	out
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

fn eat_identifier(start: char, iter: &mut Peekable<Chars>) -> Option<Token>
{
	let mut value;

	if !is_identifier_start(start) {
		return None;
	}

	value = start.to_string();

	while let Some(&next) = iter.peek() {
		if !is_identifier_continue(next) {
			break;
		}

		value.push(next);
		iter.next();
	}

	if is_keyword(&value) {
		Some(Token::Keyword(value))
	}
	else {
		Some(Token::Identifier(value))
	}
}

fn eat_punctuator(start: char, iter: &mut Peekable<Chars>) -> Option<Token>
{
	let mut current;
	let mut new;

	if !is_punctuation(start) {
		return None;
	}

	current = start.to_string();

	while let Some(&next) = iter.peek() {
		new = current.clone() + &next.to_string();

		if !is_compound_punctuator(&new) {
			break;
		}

		current = new;
		iter.next();
	}

	Some(Token::Punctuator(current))
}

fn eat_integer(start: char, iter: &mut Peekable<Chars>) -> Option<Token>
{
	let mut value;

	if !start.is_ascii_digit() {
		return None;
	}

	value = start as u8 - b'0';

	while let Some(&next) = iter.peek() {
		if !next.is_ascii_digit() {
			break;
		}

		value *= 10;
		value += next as u8 - b'0';
		iter.next();
	}

	Some(Token::Integer(value.into()))
}
