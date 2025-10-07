use crate::output::Code;

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

const ET_EXEC: Elf64Half = 2;
const EM_X86_64: Elf64Half = 62;
const SHN_UNDEF: Elf64Half = 0;

const PT_LOAD: Elf64Word = 1;

const PF_X: Elf64Word = 0b001;
const PF_R: Elf64Word = 0b100;

const EHDR_SIZE: u16 = 64;
const PHDR_SIZE: u16 = 56;
const FILE_OFFSET: u64 = (EHDR_SIZE + PHDR_SIZE) as u64;
const VADDR: u64 = 0x100_000 + FILE_OFFSET;

pub fn construct_elf(mut code: Code) -> Vec<u8>
{
	let mut out = Vec::new();

	write_file_header(code.entrypoint, &mut out);
	write_program_header(code.text.len(), &mut out);
	out.append(&mut code.text);

	out
}

fn as_bytes<T>(x: &T) -> &[u8]
{
	unsafe { std::slice::from_raw_parts(std::ptr::from_ref(x).cast(), size_of::<T>()) }
}

fn write_file_header(entrypoint: usize, out: &mut Vec<u8>)
{
	let mut e_ident = [0; 16];
	let file_header;

	e_ident[EI_MAG0] = b'\x7f';
	e_ident[EI_MAG1] = b'E';
	e_ident[EI_MAG2] = b'L';
	e_ident[EI_MAG3] = b'F';

	e_ident[EI_CLASS] = ELFCLASS64;
	e_ident[EI_DATA] = ELFDATA2LSB;
	e_ident[EI_VERSION] = EV_CURRENT;
	e_ident[EI_OSABI] = ELFOSABI_SYSV;

	file_header = Elf64Ehdr {
		e_ident,
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
