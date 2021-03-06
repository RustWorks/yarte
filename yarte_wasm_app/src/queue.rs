//! A mostly lock-free single-producer, single consumer queue.
//!
// http://www.1024cores.net/home/lock-free-algorithms/queues/non-intrusive-mpsc-node-based-queue

// TODO: test grow array implementation
use core::{cell::UnsafeCell, ptr};

use alloc::boxed::Box;

use super::unwrap;

#[derive(Debug)]
struct Node<T> {
    next: UnsafeCell<*mut Node<T>>,
    value: Option<T>,
}

/// This Queue is unsafe because only one thread can use it and it need checks atomicy at `.pop()`
#[derive(Debug)]
pub struct Queue<T> {
    head: UnsafeCell<*mut Node<T>>,
    tail: UnsafeCell<*mut Node<T>>,
}

impl<T> Node<T> {
    unsafe fn new(v: Option<T>) -> *mut Node<T> {
        Box::into_raw(Box::new(Node {
            next: UnsafeCell::new(ptr::null_mut()),
            value: v,
        }))
    }
}

impl<T> Queue<T> {
    /// Creates a new queue
    pub fn new() -> Queue<T> {
        let stub = unsafe { Node::new(None) };
        Queue {
            head: UnsafeCell::new(stub),
            tail: UnsafeCell::new(stub),
        }
    }

    /// Pushes a new value onto this queue.
    pub fn push(&self, t: T) {
        unsafe {
            let n = Node::new(Some(t));
            let prev = self.head.get().replace(n);
            *(*prev).next.get() = n;
        }
    }

    /// Pops some data from this queue.
    pub fn pop(&self) -> Option<T> {
        unsafe {
            let tail = *self.tail.get();
            let next = *(*tail).next.get();

            if next.is_null() {
                None
            } else {
                *self.tail.get() = next;
                debug_assert!((*tail).value.is_none());
                debug_assert!((*next).value.is_some());
                let ret = unwrap((*next).value.take());
                let _ = Box::from_raw(tail);
                Some(ret)
            }
        }
    }
}

#[cfg(debug_assertions)]
impl<T> Drop for Queue<T> {
    fn drop(&mut self) {
        unsafe {
            let mut cur = *self.tail.get();
            while !cur.is_null() {
                let next = (*cur).next.get();
                drop(Box::from_raw(cur));
                cur = *next;
            }
        }
    }
}

#[cfg(test)]
mod test {
    use alloc::rc::Rc;
    use wasm_bindgen_futures::spawn_local;
    use wasm_bindgen_test::*;

    use super::*;

    #[wasm_bindgen_test]
    fn test() {
        let q = Rc::new(Queue::new());
        let q1 = Rc::clone(&q);

        spawn_local(async move {
            for i in 0..100_000 {
                loop {
                    match q1.pop() {
                        Some(j) if i == j => break,
                        Some(_) => panic!(),
                        None => {}
                    }
                }
            }
            assert!(q1.pop().is_none());
        });
        for i in 0..100_000 {
            q.push(i);
        }
    }

    #[wasm_bindgen_test]
    fn test_e() {
        let q = &Queue::new();

        for i in 0..100 {
            q.push(i);
        }
        for i in 0..100 {
            loop {
                match q.pop() {
                    Some(j) if i == j => break,
                    Some(_) => panic!(),
                    None => {}
                }
            }
        }
        assert!(q.pop().is_none());
    }
}
