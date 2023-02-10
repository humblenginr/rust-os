use core::{
    pin::Pin,
    task::{Context, Poll},
};

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};

use crate::{print, println};
use futures_util::{stream::Stream, StreamExt};

// since ArrayQueue::init() does heap allocation, we cannot initialize this as a static variable.
// We could have used lazy_static, but he we are using OnceCell from `conquer_once` crate.
static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();

use futures_util::task::AtomicWaker;

static WAKER: AtomicWaker = AtomicWaker::new();

// Called by the keyboard interrupt handler
// We are not initializing the SCANCODE_QUEUE here because this will be called by the interrupt
// handler, which should not perform heap allocations
//
// Must not block or allocate
//
// `pub(crate)` makes it visible only to lib.rs since we dont want this function to be used in
// main.rs
pub(crate) fn add_scancode(scan_code: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scan_code) {
            println!("WARNING: scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        println!("WARNING: scancode queue uninitialized; dropping keyboard input");
    }
}

pub struct ScancodeStream {
    // this private field is there just to make sure that this struct cannot be initialized outside
    // of this module. The only way to initialize this struct would be to use it's `new` method
    _private: (),
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream { _private: () }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        // fast path
        if let Ok(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(&cx.waker());
        match queue.pop() {
            Ok(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            Err(crossbeam_queue::PopError) => Poll::Pending,
        }
    }
}

pub async fn print_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(layouts::Us104Key, ScancodeSet1, HandleControl::Ignore);

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if let Some(key) = keyboard.process_keyevent(key_event) {
                match key {
                    DecodedKey::Unicode(character) => print!("{}", character),
                    DecodedKey::RawKey(key) => print!("{:?}", key),
                }
            }
        }
    }
}
