// this binary has to be built for a target that does not have an underlying operating system
// it also means that the compiler and the linker should not expect the presence of a C runtime(usually crt0) - this is done by changing the build target

// we are disabling the standard libray by using this attribute
// we won't have access to things like file system, threads, networking etc.
#![no_std]
// the execution of any program does not directly start with main
// a `runtime system` is initalized before that
// in case of rust, `crt0`, a C runtime is initialized which does things like creating stack, placing arguments in the register etc.
// after that, it will call the rust entry point(not the `main` function), which will do some minimal things and call `main`
// but for a free-standing binray, we don't have acces to `crt0`
// we are using this attribute to tell the rust compiler that we don't want the normal entry point chain
#![no_main]

use core::panic::PanicInfo;

use rust_os::println;

// we have removed the default `main` function since we don't have the underlying runtime system to call it
// we now overwrite this with our own `_start` function
// [no_mangle] makes sure that the compiler keeps the name as it is and doesn't change it(usually it adds some random strings to make it unique)
// we use `_start` as the name because that's what most of the systems have as their default entry point
#[no_mangle]
// extern "C" here means that the function should be called with C calling convention
pub extern "C" fn _start() -> ! {
    rust_os::init();
    loop {}
}

// panic_handler, as the name suggests, is what knows how to handle a `panic`
// this is needed as we have disabled the standard library
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{}", info);
    loop {}
}
