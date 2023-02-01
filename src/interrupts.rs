use crate::{gdt, println};
use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    // this will only initialize the first time IDT is referenced
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault.set_handler_fn(double_fault_handler)
                // Currently, we only have a single Stack. Whevener a page fault occurs due to stack overflow,
                // the exception handler will try to push a `InterruptStackFrame` onto the stack and it will
                // trigger a double fault (since the stack has already overflowed). Now, the double fault will try
                // to push the `InterruptStackFrame` onto the stack and it will trigger the `triple fault` for the
                // same reason. We need to prevent this. So, we will be switching to a different stack whenever a
                // double fault occurs.
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // new
        }
        idt
    };
}

// Interrupt descriptor table is what the CPU uses to find
// the handlers for all sorts of exceptions
pub fn init_idt() {
    IDT.load();
}

// the x86-interrupt calling convention makes sure that all the
// registers are preserved. A calling convention divides the existing registers into
// two categories - 'preserved regsiters' and 'scratch registers'
// The calling convention ensures that the `preserved registers` are not modified when the
// function returns, whereas the `scratch registers` can be modified.
// This is fine for normal function calls which happen only with the `call` instruction
// but in case of exceptions, it can happen at any instruction and so there is a need to make sure
// that all the registers are preserved - which is done by `x86-interrupt` calling convention
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    println!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
