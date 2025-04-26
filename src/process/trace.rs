use std::{
    sync::mpsc::{Sender, channel},
    thread::{self, JoinHandle},
};

pub(super) struct Trace<T> {
    sender: Sender<T>,
    handle: JoinHandle<Vec<T>>,
}

impl<T> Trace<T> {
    pub(super) fn sender(&self) -> Sender<T> {
        self.sender.clone()
    }

    pub(super) fn finish(self) -> Vec<T> {
        drop(self.sender);
        self.handle.join().expect("oops! the child thread panicked")
    }
}

pub(super) fn tracer<T: Send + 'static>() -> Trace<T> {
    let (sender, receiver) = channel();

    // Collect the name or id for the path taken
    let recv_handle = thread::spawn(move || {
        let mut trace = vec![];
        while let Ok(value) = receiver.recv() {
            trace.push(value);
        }
        trace
    });
    Trace {
        sender,
        handle: recv_handle,
    }
}
