use std::fmt::Display;

use crate::args::ARGS;
use crate::lexer::{Token, TokenType, token_text};
use crate::utils::error;

pub struct AST
{
	ty: Type,
	next: Vec<AST>,
	data: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub enum Type
{
	AdditiveExpression,
	AndExpression,
	ArgumentExpressionList,
	AssignmentExpression,
	BlockItem,
	CastExpression,
	CompoundStatement,
	ConditionalExpression,
	Constant,
	DeclarationSpecifiers,
	Declarator,
	EqualityExpression,
	ExclusiveOrExpression,
	Expression,
	ExpressionStatement,
	ExternalDeclaration,
	FunctionDeclarator,
	FunctionDefinition,
	Identifier,
	InclusiveOrExpression,
	JumpStatement,
	LogicalAndExpression,
	LogicalOrExpression,
	MultiplicativeExpression,
	ParameterDeclaration,
	ParameterList,
	PostfixExpression,
	PrimaryExpression,
	RelationalExpression,
	ShiftExpression,
	TranslationUnit,
	TypeSpecifier,
	UnaryExpression,
	UnlabeledStatement,
}

use Type::*;

#[derive(Clone, Copy)]
struct TokenReader<'file>
{
	filename: &'file str,
	input: &'file str,
	tokens: &'file [Token],
	idx: usize,
}

fn ast_node(read: &TokenReader, ty: Type) -> AST
{
	reader_dbg(read, ty);

	AST { ty, next: Vec::new(), data: None }
}

fn ast_with_data(read: &TokenReader, ty: Type, data: String) -> AST
{
	reader_dbg(read, ty);

	AST { ty, next: Vec::new(), data: Some(data) }
}

fn reader_curr<'f>(read: &TokenReader<'f>) -> &'f Token
{
	&read.tokens[read.idx]
}

fn reader_dbg(read: &TokenReader, ast_type: Type)
{
	let (_token_type, text) = reader_data(read);
	let idx = read.idx;
	let len = read.tokens.len();

	if !ARGS.verbose {
		return;
	}

	println!("{ast_type:?} [{idx}/{len}] {text}");
}

fn reader_advance(read: &mut TokenReader)
{
	let (ty, text) = reader_data(read);

	if ty == TokenType::EOF {
		return;
	}

	if ARGS.verbose {
		println!("consume {text:?}");
	}

	read.idx += 1;
}

fn reader_data<'f>(read: &TokenReader<'f>) -> (TokenType, &'f str)
{
	let curr = reader_curr(read);
	let text = token_text(curr, read.input);

	(curr.ty, text)
}

fn reader_matches(read: &TokenReader, token_type: TokenType, target: &str) -> bool
{
	let (ty, text) = reader_data(read);

	ty == token_type && text == target
}

fn reader_optional(read: &mut TokenReader, token_type: TokenType, target: &str) -> Option<()>
{
	let (ty, text) = reader_data(read);

	if ty == token_type && text == target {
		reader_advance(read);

		return Some(());
	}

	None
}

fn reader_eat_any(read: &mut TokenReader, token_type: TokenType, items: &[&str]) -> Option<String>
{
	let (ty, text) = reader_data(read);

	if ty != token_type {
		return None;
	}

	for &item in items {
		if text == item {
			reader_advance(read);

			return Some(item.to_string());
		}
	}

	None
}

fn reader_expect(read: &mut TokenReader, token_type: TokenType, target: &str)
{
	let (_ty, text) = reader_data(read);

	if reader_optional(read, token_type, target).is_none() {
		parse_error(read, format!("unexpected token {text:?}, expected {target:?}"));
	}
}

fn parse_error(read: &TokenReader, msg: impl Display) -> !
{
	let mut message = format!("{msg}");
	let token = reader_curr(read);
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

fn match_keyword(read: &TokenReader, keywords: &[&str]) -> Option<String>
{
	let curr = reader_curr(read);
	let text;

	if curr.ty != TokenType::Keyword {
		return None;
	}

	text = token_text(curr, read.input);

	for &keyword in keywords {
		if text == keyword {
			return Some(keyword.to_string());
		}
	}

	None
}

pub fn print_ast(ast: &AST)
{
	print_ast_rec(ast, 0);
}

fn print_ast_rec(ast: &AST, level: usize)
{
	let indent = " ".repeat(2 * level);
	let data = match &ast.data {
		Some(data) => &format!(" {data:?}"),
		None => "",
	};

	println!("{indent}{:?}{data} [", ast.ty);

	for next in &ast.next {
		print_ast_rec(next, level + 1);
	}

	println!("{indent}]");
}

pub fn parse(filename: &str, input: &str, tokens: &[Token]) -> AST
{
	let mut read = TokenReader { filename, input, tokens, idx: 0 };
	let ast;

	ast = translation_unit(&mut read);

	if reader_curr(&read).ty != TokenType::EOF {
		parse_error(&read, "unexpected token at EOF");
	}

	ast
}

fn translation_unit(read: &mut TokenReader) -> AST
{
	let mut node = ast_node(read, TranslationUnit);

	while let Some(next) = external_declaration(read) {
		node.next.push(next);
	}

	node
}

fn external_declaration(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ExternalDeclaration);
	let mut copy = *read;

	if let Some(next) = function_definition(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	None
}

fn function_definition(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, FunctionDefinition);

	node.next.push(declaration_specifiers(read)?);
	node.next.push(declarator(read)?);
	node.next.push(compound_statement(read)?);

	Some(node)
}

fn declaration_specifiers(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, DeclarationSpecifiers);

	node.next.push(type_specifier(read)?);

	Some(node)
}

fn type_specifier(read: &mut TokenReader) -> Option<AST>
{
	let keywords =
		["void", "char", "short", "int", "long", "float", "double", "signed", "unsigned"];

	if let Some(keyword) = match_keyword(read, &keywords) {
		reader_advance(read);

		return Some(ast_with_data(read, TypeSpecifier, keyword));
	}

	None
}

fn declarator(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, Declarator);
	let mut copy;

	copy = *read;

	if let Some(next) = function_declarator(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	copy = *read;

	if let Some(next) = identifier(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	None
}

fn function_declarator(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, FunctionDeclarator);

	node.next.push(identifier(read)?);

	reader_optional(read, TokenType::Punctuator, "(")?;

	node.next.push(parameter_list(read));

	reader_expect(read, TokenType::Punctuator, ")");

	Some(node)
}

fn identifier(read: &mut TokenReader) -> Option<AST>
{
	let (ty, text) = reader_data(read);

	if ty == TokenType::Identifier {
		reader_advance(read);

		return Some(ast_with_data(read, Identifier, text.to_string()));
	}

	None
}

fn constant(read: &mut TokenReader) -> Option<AST>
{
	let (ty, text) = reader_data(read);

	if ty == TokenType::Integer {
		reader_advance(read);

		return Some(ast_with_data(read, Constant, text.to_string()));
	}

	None
}

fn parameter_list(read: &mut TokenReader) -> AST
{
	let mut node = ast_node(read, ParameterList);

	while let Some(next) = parameter_declaration(read) {
		node.next.push(next);

		if reader_matches(read, TokenType::Punctuator, ",") {
			reader_advance(read);
			continue;
		}

		break;
	}

	node
}

fn parameter_declaration(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ParameterDeclaration);

	node.next.push(declaration_specifiers(read)?);

	if let Some(next) = declarator(read) {
		node.next.push(next);
		return Some(node);
	}

	Some(node)
}

fn compound_statement(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, CompoundStatement);

	reader_optional(read, TokenType::Punctuator, "{")?;

	while let Some(next) = block_item(read) {
		node.next.push(next);
	}

	reader_expect(read, TokenType::Punctuator, "}");

	Some(node)
}

fn block_item(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, BlockItem);
	let mut copy;

	copy = *read;

	if let Some(next) = unlabeled_statement(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	None
}

fn unlabeled_statement(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, UnlabeledStatement);
	let mut copy;

	copy = *read;

	if let Some(next) = expression_statement(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	copy = *read;

	if let Some(next) = jump_statement(&mut copy) {
		node.next.push(next);
		*read = copy;
		return Some(node);
	}

	None
}

fn expression_statement(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ExpressionStatement);

	if reader_optional(read, TokenType::Punctuator, ";").is_some() {
		return Some(node);
	}

	if let Some(next) = expression(read) {
		node.next.push(next);

		reader_expect(read, TokenType::Punctuator, ";");

		return Some(node);
	}

	None
}

fn expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, Expression);

	node.next.push(assignment_expression(read)?);

	while reader_optional(read, TokenType::Punctuator, ",").is_some() {
		node.next.push(assignment_expression(read)?);
	}

	Some(node)
}

fn assignment_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, AssignmentExpression);

	if let Some(next) = conditional_expression(read) {
		node.next.push(next);
		return Some(node);
	}

	None
}

fn conditional_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ConditionalExpression);

	node.next.push(logical_or_expression(read)?);

	Some(node)
}

fn logical_or_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, LogicalOrExpression);

	node.next.push(logical_and_expression(read)?);

	Some(node)
}

fn logical_and_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, LogicalAndExpression);

	node.next.push(inclusive_or_expression(read)?);

	Some(node)
}

fn inclusive_or_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, InclusiveOrExpression);

	node.next.push(exclusive_or_expression(read)?);

	Some(node)
}

fn exclusive_or_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ExclusiveOrExpression);

	node.next.push(and_expression(read)?);

	Some(node)
}

fn and_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, AndExpression);

	node.next.push(equality_expression(read)?);

	Some(node)
}

fn equality_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, EqualityExpression);

	node.next.push(relational_expression(read)?);

	Some(node)
}

fn relational_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, RelationalExpression);

	node.next.push(shift_expression(read)?);

	Some(node)
}

fn shift_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ShiftExpression);

	node.next.push(additive_expression(read)?);

	Some(node)
}

fn additive_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, AdditiveExpression);

	node.next.push(multiplicative_expression(read)?);

	if let Some(data) = reader_eat_any(read, TokenType::Punctuator, &["+", "-"]) {
		node.data = Some(data);
		node.next.push(multiplicative_expression(read)?);
	}

	Some(node)
}

fn multiplicative_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, MultiplicativeExpression);

	node.next.push(cast_expression(read)?);

	Some(node)
}

fn cast_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, CastExpression);

	node.next.push(unary_expression(read)?);

	Some(node)
}

fn unary_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, UnaryExpression);

	node.next.push(postfix_expression(read)?);

	Some(node)
}

fn postfix_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, PostfixExpression);

	node.next.push(primary_expression(read)?);

	if reader_optional(read, TokenType::Punctuator, "(").is_some() {
		node.data = Some("(".to_string());

		if let Some(next) = argument_expression_list(read) {
			node.next.push(next);
		}

		reader_expect(read, TokenType::Punctuator, ")");
	}

	Some(node)
}

fn argument_expression_list(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, ArgumentExpressionList);

	node.next.push(assignment_expression(read)?);

	while reader_optional(read, TokenType::Punctuator, ",").is_some() {
		node.next.push(assignment_expression(read)?);
	}

	Some(node)
}

fn primary_expression(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, PrimaryExpression);

	if let Some(next) = identifier(read) {
		node.next.push(next);
		return Some(node);
	}

	if let Some(next) = constant(read) {
		node.next.push(next);
		return Some(node);
	}

	None
}

fn jump_statement(read: &mut TokenReader) -> Option<AST>
{
	let mut node = ast_node(read, JumpStatement);

	reader_optional(read, TokenType::Keyword, "return")?;

	if let Some(next) = expression(read) {
		node.next.push(next);
	}

	reader_expect(read, TokenType::Punctuator, ";");

	Some(node)
}
