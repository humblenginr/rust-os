use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use alloc::boxed::Box;

pub mod keyboard;
pub mod simple_executor;

pub struct Task {
    // dyn keyword allows us to set the type parameter of the Box as anything that implements the
    // Future trais (trait objects). Rust will use dynamic dispatch (where the methods to be called
    // are calculated at runtime) for calling the methods of the trait object
    //
    // we use Pin because the futures created by async/await might be self-referential
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {
    // by using 'static here, I am affirming that the `future` will be valid for the whole lifetime
    // of the program
    pub fn new(future: impl Future<Output = ()> + 'static) -> Self {
        Task {
            future: Box::pin(future),
        }
    }
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        // we use .as_mut because the `poll` method of the future requires to be called on
        // Pin<&mutT>
        self.future.as_mut().poll(context)
    }
}
