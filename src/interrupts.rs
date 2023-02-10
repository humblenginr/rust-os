use crate::task::keyboard::add_scancode;
use crate::{gdt, println};
use crate::{hlt_loop, print};
use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin;
use x86_64::instructions::port::Port;
use x86_64::structures::idt::{
    InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode, PageFaultHandlerFunc,
};

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptsIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptsIndex {
    fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

// PICs by default are configured to send interrupt codes starting from 1 which will conflict with
// the system defined interrupts in the IDT (like double fault for 8, etc.)
// so we set the offset to 32 because that is where the system defined interrupts end
pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

// There will be two PICs (Programmable Interrupt Controllers), primary and secondary, and they will be
// connected to the I/O Ports. This crate (pic8259) is just an abstraction for working with
// the PICs.
pub static PICS: spin::Mutex<ChainedPics> =
    spin::Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

lazy_static! {
    // this will only initialize the first time IDT is referenced
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt[InterruptsIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptsIndex::Keyboard.as_usize()].set_handler_fn(keypress_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
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

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    println!("EXCEPTION: PAGE FAULT");
    // CR2 register is automatically set up by the operating system and contains the virtual
    // address that caused the page fault
    println!("ACCESSED ADDRESS: {:?}", Cr2::read());
    println!("Error Code: {:?}", error_code);
    println!("{:#?}", stack_frame);
    hlt_loop();
}

extern "x86-interrupt" fn keypress_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
    use spin::Mutex;

    // creating a static Kebyboard reference
    lazy_static! {
        static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
            Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore)
        );
    };
    let mut keyboard = KEYBOARD.lock();

    // As soon as you press something, the keyboard controller will send the keypress data in the
    // 0x60 PS/2 port and then trigger the interrupt, now unless the data from the PS/2 port is
    // read, it will not send any more interrupts
    let mut port = Port::new(0x60);
    // we read the data from the port
    let scan_code: u8 = unsafe { port.read() };
    // adding the scan_code to the task queue
    add_scancode(scan_code);

    // the PIC expects us to send an `end of interrupt (EOI)` signal from the handler
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptsIndex::Keyboard.as_u8());
    }
}

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    print!(".");
    // the PIC expects us to send an `end of interrupt (EOI)` signal from the handler
    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptsIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
}
