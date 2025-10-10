use std::collections::HashMap;

use crate::output::Code;
use crate::utils::CheckError;

#[repr(C, packed)]
struct Elf64Ehdr
{
	e_ident: [u8; 16],
	e_type: Elf64Half,
	e_machine: Elf64Half,
	e_version: Elf64Word,
	e_entry: Elf64Addr,
	e_phoff: Elf64Off,
	e_shoff: Elf64Off,
	e_flags: Elf64Word,
	e_ehsize: Elf64Half,
	e_phentsize: Elf64Half,
	e_phnum: Elf64Half,
	e_shentsize: Elf64Half,
	e_shnum: Elf64Half,
	e_shstrndx: Elf64Half,
}

#[repr(C, packed)]
#[derive(Default)]
struct Elf64Shdr
{
	sh_name: Elf64Word,
	sh_type: Elf64Word,
	sh_flags: Elf64Xword,
	sh_addr: Elf64Addr,
	sh_offset: Elf64Off,
	sh_size: Elf64Xword,
	sh_link: Elf64Word,
	sh_info: Elf64Word,
	sh_addralign: Elf64Xword,
	sh_entsize: Elf64Xword,
}

#[repr(C, packed)]
#[derive(Default)]
struct Elf64Sym
{
	st_name: Elf64Word,
	st_info: u8,
	st_other: u8,
	st_shndx: Elf64Half,
	st_value: Elf64Addr,
	st_size: Elf64Xword,
}

#[repr(C, packed)]
struct Elf64Phdr
{
	p_type: Elf64Word,
	p_flags: Elf64Word,
	p_offset: Elf64Off,
	p_vaddr: Elf64Addr,
	p_paddr: Elf64Addr,
	p_filesz: Elf64Xword,
	p_memsz: Elf64Xword,
	p_align: Elf64Xword,
}

struct StringTable
{
	bytes: Vec<u8>,
	offsets: HashMap<String, usize>,
}

type Elf64Addr = u64;
type Elf64Off = u64;
type Elf64Half = u16;
type Elf64Word = u32;
type Elf64Xword = u64;

const EI_MAG0: usize = 0;
const EI_MAG1: usize = 1;
const EI_MAG2: usize = 2;
const EI_MAG3: usize = 3;
const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const EI_VERSION: usize = 6;
const EI_OSABI: usize = 7;

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const EV_CURRENT: u8 = 1;
const ELFOSABI_SYSV: u8 = 0;

const ET_REL: Elf64Half = 1;
const ET_EXEC: Elf64Half = 2;
const EM_X86_64: Elf64Half = 62;
const SHN_UNDEF: Elf64Half = 0;

const PT_LOAD: Elf64Word = 1;

const PF_X: Elf64Word = 0b001;
const PF_R: Elf64Word = 0b100;

const EHDR_SIZE: u16 = 64;
const SHDR_SIZE: u16 = 64;
const PHDR_SIZE: u16 = 56;
const SYMT_SIZE: u16 = 24;

const FILE_OFFSET: u64 = (EHDR_SIZE + PHDR_SIZE) as u64;
const VADDR: u64 = 0x100_000 + FILE_OFFSET;

const SHT_PROGBITS: Elf64Word = 1;
const SHT_SYMTAB: Elf64Word = 2;
const SHT_STRTAB: Elf64Word = 3;

const SHF_ALLOC: Elf64Xword = 2;
const SHF_EXECINSTR: Elf64Xword = 4;

const STB_GLOBAL: u8 = 1;
const STT_NOTYPE: u8 = 0;

const SHNDX_TEXT: u16 = 4;

pub fn construct_object_file(code: Code) -> Vec<u8>
{
	let num_shdrs = 5;
	let mut out = Vec::new();

	write_file_header_objfile(num_shdrs, &mut out);
	write_shdrs_and_sections(code, num_shdrs, &mut out);

	out
}

pub fn construct_executable(mut inputs: Vec<Code>) -> Vec<u8>
{
	let mut out = Vec::new();

	let [code] = inputs.as_mut_slice()
	else {
		todo!()
	};

	write_file_header_exec(code.entrypoint, &mut out);
	write_program_header(code.text.len(), &mut out);
	out.append(&mut code.text);

	out
}

fn as_bytes<T>(x: &T) -> &[u8]
{
	unsafe { std::slice::from_raw_parts(std::ptr::from_ref(x).cast(), size_of::<T>()) }
}

fn write_file_header_objfile(num_shdrs: u16, out: &mut Vec<u8>)
{
	let file_header = Elf64Ehdr {
		e_ident: elf_ident(),
		e_type: ET_REL,
		e_machine: EM_X86_64,
		e_version: EV_CURRENT.into(),
		e_entry: 0,
		e_phoff: 0,
		e_shoff: EHDR_SIZE.into(),
		e_flags: 0,
		e_ehsize: EHDR_SIZE,
		e_phentsize: 0,
		e_phnum: 0,
		e_shentsize: SHDR_SIZE,
		e_shnum: num_shdrs,
		e_shstrndx: 1,
	};

	out.extend(as_bytes(&file_header));
}

fn write_file_header_exec(entrypoint: usize, out: &mut Vec<u8>)
{
	let file_header = Elf64Ehdr {
		e_ident: elf_ident(),
		e_type: ET_EXEC,
		e_machine: EM_X86_64,
		e_version: EV_CURRENT.into(),
		e_entry: VADDR + entrypoint as u64,
		e_phoff: EHDR_SIZE.into(),
		e_shoff: 0,
		e_flags: 0,
		e_ehsize: EHDR_SIZE,
		e_phentsize: PHDR_SIZE,
		e_phnum: 1,
		e_shentsize: 0,
		e_shnum: 0,
		e_shstrndx: SHN_UNDEF,
	};

	out.extend(as_bytes(&file_header));
}

fn elf_ident() -> [u8; 16]
{
	let mut ident = [0; 16];

	ident[EI_MAG0] = b'\x7f';
	ident[EI_MAG1] = b'E';
	ident[EI_MAG2] = b'L';
	ident[EI_MAG3] = b'F';

	ident[EI_CLASS] = ELFCLASS64;
	ident[EI_DATA] = ELFDATA2LSB;
	ident[EI_VERSION] = EV_CURRENT;
	ident[EI_OSABI] = ELFOSABI_SYSV;

	ident
}

fn write_shdrs_and_sections(mut code: Code, num_shdrs: u16, out: &mut Vec<u8>)
{
	let mut shdr_st = strtab_new();
	let mut symb_st = strtab_new();
	let symtab;

	let mut offset;
	let shstrtab_len;
	let strtab_len;
	let symtab_len;
	let text_len;

	strtab_add_many(&mut shdr_st, &[".shstrtab", ".strtab", ".symtab", ".text"]);
	strtab_add_symbols(&mut symb_st, &code.globals);
	symtab = construct_symtab(&code.globals, &symb_st);

	shstrtab_len = shdr_st.bytes.len() as u64;
	strtab_len = symb_st.bytes.len() as u64;
	symtab_len = symtab.len() as u64 * u64::from(SYMT_SIZE);
	text_len = code.text.len() as u64;

	out.extend(as_bytes(&Elf64Shdr::default()));

	offset = (EHDR_SIZE + SHDR_SIZE * num_shdrs).into();

	out.extend(as_bytes(&Elf64Shdr {
		sh_name: strtab_get(&shdr_st, ".shstrtab"),
		sh_type: SHT_STRTAB,
		sh_flags: 0,
		sh_addr: 0,
		sh_offset: offset,
		sh_size: shstrtab_len,
		sh_link: 0,
		sh_info: 0,
		sh_addralign: 1,
		sh_entsize: 0,
	}));

	offset += shstrtab_len;

	out.extend(as_bytes(&Elf64Shdr {
		sh_name: strtab_get(&shdr_st, ".strtab"),
		sh_type: SHT_STRTAB,
		sh_flags: 0,
		sh_addr: 0,
		sh_offset: offset,
		sh_size: strtab_len,
		sh_link: 0,
		sh_info: 0,
		sh_addralign: 1,
		sh_entsize: 0,
	}));

	offset += strtab_len;
	offset = offset.next_multiple_of(16);

	out.extend(as_bytes(&Elf64Shdr {
		sh_name: strtab_get(&shdr_st, ".symtab"),
		sh_type: SHT_SYMTAB,
		sh_flags: 0,
		sh_addr: 0,
		sh_offset: offset,
		sh_size: symtab_len,
		sh_link: 2,
		sh_info: 1,
		sh_addralign: 16,
		sh_entsize: SYMT_SIZE.into(),
	}));

	offset += symtab_len;
	offset = offset.next_multiple_of(16);

	out.extend(as_bytes(&Elf64Shdr {
		sh_name: strtab_get(&shdr_st, ".text"),
		sh_type: SHT_PROGBITS,
		sh_flags: SHF_ALLOC | SHF_EXECINSTR,
		sh_addr: 0,
		sh_offset: offset,
		sh_size: text_len,
		sh_link: 0,
		sh_info: 0,
		sh_addralign: 16,
		sh_entsize: 0,
	}));

	out.extend(shdr_st.bytes);
	out.extend(symb_st.bytes);

	pad_until_multiple_of(out, 16);

	for sym in symtab {
		out.extend(as_bytes(&sym));
	}

	pad_until_multiple_of(out, 16);

	out.append(&mut code.text);
}

fn pad_until_multiple_of(out: &mut Vec<u8>, mult: usize)
{
	while !out.len().is_multiple_of(mult) {
		out.push(0);
	}
}

fn write_program_header(size: usize, out: &mut Vec<u8>)
{
	let offset = (EHDR_SIZE + PHDR_SIZE).into();
	let size = size as u64;
	let prog_header = Elf64Phdr {
		p_type: PT_LOAD,
		p_flags: PF_R | PF_X,
		p_offset: offset,
		p_vaddr: VADDR,
		p_paddr: 0,
		p_filesz: size,
		p_memsz: size,
		p_align: 0,
	};

	out.extend(as_bytes(&prog_header));
}

fn strtab_new() -> StringTable
{
	StringTable { bytes: vec![0], offsets: HashMap::new() }
}

fn strtab_add_many(tab: &mut StringTable, items: &[&str])
{
	for item in items {
		strtab_add(tab, item);
	}
}

fn strtab_add(tab: &mut StringTable, item: &str)
{
	let offset = tab.bytes.len();

	tab.bytes.extend_from_slice(item.as_bytes());
	tab.bytes.push(0);

	tab.offsets.insert(item.to_string(), offset);
}

fn strtab_add_symbols(tab: &mut StringTable, globals: &[(String, usize)])
{
	for (symbol, _addr) in globals {
		strtab_add(tab, symbol);
	}
}

fn strtab_get(tab: &StringTable, item: &str) -> u32
{
	let idx = *tab.offsets.get(item).try_to(format!("find symbol {item:?} in string table"));

	u32::try_from(idx).or_err("index overflows u32")
}

fn construct_symtab(globals: &[(String, usize)], strtab: &StringTable) -> Vec<Elf64Sym>
{
	let mut sym;
	let mut out = Vec::new();

	out.push(Elf64Sym::default());

	for (symbol, addr) in globals {
		sym = Elf64Sym {
			st_name: strtab_get(strtab, symbol),
			st_info: info(STB_GLOBAL, STT_NOTYPE),
			st_other: 0,
			st_shndx: SHNDX_TEXT,
			st_value: *addr as u64,
			st_size: 0,
		};

		out.push(sym);
	}

	out
}

fn info(binding: u8, ty: u8) -> u8
{
	(binding << 4) | ty
}
