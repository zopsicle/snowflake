use {
    iced_x86::code_asm::{CodeAssembler, IcedError, registers},
};

pub fn f(asm: &mut CodeAssembler) -> Result<(), IcedError>
{
    let mut label_heap_alloc_failed = asm.create_label();

    asm.add(registers::rdi, 4i32)?;
    asm.cmp(registers::rdi, registers::rsi)?;
    asm.jg(label_heap_alloc_failed)?;

    asm.mov(registers::rax, 0u64)?;
    asm.ret()?;

    asm.set_label(&mut label_heap_alloc_failed)?;
    asm.mov(registers::rax, 1u64)?;
    asm.ret()?;

    Ok(())
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        iced_x86::{Decoder, Formatter, NasmFormatter},
        std::{mem::transmute, ptr::{copy_nonoverlapping, null_mut}},
    };

    #[test]
    fn example()
    {
        let mut asm = CodeAssembler::new(64).unwrap();
        f(&mut asm).unwrap();

        let ip = 0xFF00_0000_0000_0000;
        let machine_code = asm.assemble(ip).unwrap();

        let mut decoder = Decoder::with_ip(64, &machine_code, ip, 0);
        let mut formatter = NasmFormatter::new();
        formatter.options_mut().set_space_after_operand_separator(true);
        let mut assembly = String::new();
        for instruction in decoder.iter() {
            formatter.format(&instruction, &mut assembly);
            assembly.push('\n');
        }

        print!("{}", assembly);

        let f = unsafe {

            let page = libc::mmap(
                /* addr   */ null_mut(),
                /* length */ machine_code.len(),
                /* prot   */ libc::PROT_WRITE,
                /* flags  */ libc::MAP_ANONYMOUS | libc::MAP_PRIVATE,
                /* fd     */ -1,
                /* offset */ 0,
            );
            assert!(page != libc::MAP_FAILED);

            copy_nonoverlapping(
                /* src   */ machine_code.as_ptr(),
                /* dst   */ page.cast::<u8>(),
                /* count */ machine_code.len(),
            );

            let ok = libc::mprotect(
                /* addr */ page,
                /* len  */ machine_code.len(),
                /* prot */ libc::PROT_EXEC | libc::PROT_READ,
            );
            assert!(ok != -1);

            transmute::<
                *mut libc::c_void,
                extern "C" fn(usize, usize) -> usize,
            >(page)

        };

        panic!("{:?}", f(8, 9));
    }
}
