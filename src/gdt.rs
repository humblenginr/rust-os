/**
 * This module creates a new GlobalDescriptorTable (GDT), which is used for memory segmentation
 * purposes and intializes a new TaskStateSegment (TSS) which has a new entry in the
 * InterruptStackTable to be used for preventing triple faults when stack overflow occurs by
 * switching to this newly initialized stack
 */
use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

// newly created stack table index
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS};
    use x86_64::instructions::tables::load_tss;
    GDT.0.load();
    unsafe {
        // setting the new code_segment
        CS::set_reg(GDT.1.code_selector);
        // loading the new tss table
        load_tss(GDT.1.tss_selector);
    }
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // `interrupt_stack_table`, which is a part of the TSS (Task State Segment), is a table of 7 pointers to known-good stacks
        // we are creating a new Stack for DOUBLE FAULT in the memory and assigning it to the 0th index of the
        // Interrupt Stack Table.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // 4096 bytes * 5 = 20 kilobytes - size of the stack
            const STACK_SIZE: usize = 4096 * 5;
            // 8 bits is 1 one byte, so
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { &STACK });
            let stack_end = stack_start + STACK_SIZE;
            // the stack on x86 grows downwards (high address to low address) and hence we are
            // returning the top address
            stack_end
        };
        tss
    };
}

lazy_static! {
    // GlobalDescriptorTable (GDT) is the legacy standard for memory segmentation between
    // processes.
    // Nowadays Paging is used. But this is still kept in x86 architectures for backward
    // compatibility and for user-space to kernel stapce switching and some other needs.
    // We are creating a GDT and adding out TSS entry into it.
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors{ code_selector, tss_selector})
    };
}
struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}
