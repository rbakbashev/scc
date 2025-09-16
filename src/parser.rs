use std::fmt::Display;

use crate::lexer::Token;
use crate::utils::error;

struct TokenReader<'file>
{
	filename: &'file str,
	input: &'file str,
	tokens: &'file [Token],
	idx: usize,
}

fn reader_curr<'f>(read: &TokenReader<'f>) -> Option<&'f Token>
{
	read.tokens.get(read.idx)
}

fn parse_error(read: &TokenReader, msg: impl Display) -> !
{
	let mut message = format!("{msg}");
	let token = reader_curr(read).unwrap_or_else(|| error(&message));
	let (raw_line, number, start) = get_line_of_token(token, read.input);
	let (line, added) = expand_tabs(raw_line);
	let column = token.start as usize - start + 1;

	message += "\n\n";
	message += &line;
	message += &underline(token, start, added);
	message += "\n\n";
	message += &format!("at {}:{}:{}", read.filename, number, column);

	error(message)
}

fn get_line_of_token<'file>(token: &Token, input: &'file str) -> (&'file str, i32, usize)
{
	let mut start = 0;
	let mut end = 0;
	let mut number = 1;

	for (i, ch) in input.char_indices() {
		end = i;

		if ch != '\n' {
			continue;
		}

		if i > token.end as usize {
			break;
		}

		start = i + 1;
		number += 1;
	}

	(&input[start..=end], number, start)
}

fn expand_tabs(line: &str) -> (String, usize)
{
	let tab_size = 8;

	let mut out = String::new();
	let mut target;
	let mut added = 0;

	for ch in line.chars() {
		if ch != '\t' {
			out.push(ch);
			continue;
		}

		target = (out.len() + 1).next_multiple_of(tab_size);

		while out.len() != target {
			out.push(' ');
			added += 1;
		}

		added -= 1;
	}

	(out, added)
}

fn underline(token: &Token, start: usize, added: usize) -> String
{
	let spaces = token.start as usize - start + added;
	let underlines = token.end as usize - token.start as usize + 1;

	" ".repeat(spaces) + &"~".repeat(underlines)
}

pub fn parse(filename: &str, input: &str, tokens: &[Token])
{
	let read = TokenReader { filename, input, tokens, idx: 0 };

	if reader_curr(&read).is_some() {
		parse_error(&read, "unexpected token at EOF");
	}
}
