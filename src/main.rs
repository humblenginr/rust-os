// this binary has to be built for a target that does not have an underlying operating system
// it also means that the compiler and the linker should not expect the presence of a C runtime(usually crt0) - this is done by changing the build target

// we are disabling the standard libray by using this attribute
// we won't have access to things like file system, threads, networking etc.
#![no_std]
// the execution of any program does not directly start with main
// a `runtime system` is initalized before that in case of rust, `crt0`, a C runtime is initialized which does things like creating stack, placing arguments in the register etc.
// after that, it will call the rust entry point(not the `main` function), which will do some minimal things and call `main`
// but for a free-standing binray, we don't have acces to `crt0`
// we are using this attribute to tell the rust compiler that we don't want the normal entry point chain
#![no_main]

extern crate alloc;
use core::panic::PanicInfo;

use bootloader::{entry_point, BootInfo};
use rust_os::{
    allocator, hlt_loop,
    memory::{self, BootInfoFrameAllocator},
    println,
    task::{
        keyboard,
        simple_executor::{self, SimpleExecutor},
        Task,
    },
};
use x86_64::VirtAddr;

// the `entry_point` macro allows us to use this function as a normal rust function but in the
// backend it wraps it in the `_start` func with 'C' calling convention and uses `[no_mangle]`
// attribute
//
//Below are the comments that describe why the `entry_point` macro does what it does
// we have removed the default `main` function since we don't have the underlying runtime system to call it
// we now overwrite this with our own `_start` function
// [no_mangle] makes sure that the compiler keeps the name as it is and doesn't change it(usually it adds some random strings to make it unique)
// we use `_start` as the name because that's what most of the systems have as their default entry point
// extern "C" here means that the function should be called with C calling convention
entry_point!(kernel_main);
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    rust_os::init();

    // Initialize Heap
    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };
    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let mut executor = SimpleExecutor::new();
    executor.spawn(Task::new(keyboard::print_keypresses())); // new
    executor.run();

    hlt_loop();
}

// panic_handler, as the name suggests, is what knows how to handle a `panic`
// this is needed as we have disabled the standard library
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    hlt_loop();
}
