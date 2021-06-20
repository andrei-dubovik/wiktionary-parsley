// Copyright (c) 2019-2020, Andrey Dubovik <andrei@dubovik.eu>

// A rudimentary doubly linked list that exposes unsafe pointers to its nodes

use std::fmt::{self, Debug};
use std::ptr;

use serde::ser::{Serialize, Serializer, SerializeSeq};

macro_rules! malloc {
    ( $expr:expr ) => { Box::into_raw(Box::new($expr)) }
}

macro_rules! free {
    ( $expr:expr ) => { Box::from_raw($expr) }
}


// Node

pub struct Node<T> {
    value: Option<T>,
    next: *mut Node<T>,
    prev: *mut Node<T>,
}


impl<T> Node<T> {
    pub fn new() -> Self {
        Node {
            value: None,
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
        }
    }

    pub fn as_mut(&mut self) -> &mut T {
        self.value.as_mut().unwrap()
    }

    pub fn take(&mut self) -> T {
        unsafe {
            let p_node = (*self.prev).next;
            (*self.prev).next = self.next;
            (*self.next).prev = self.prev;
            free!(p_node).value.unwrap()
        }
    }

    pub fn insert_before(&mut self, value: T) -> *mut Node<T> {
        let node: *mut Node<T> = malloc!(Node {
            value: Some(value),
            next: unsafe { (*self.prev).next },
            prev: self.prev,
        });
        unsafe { (*self.prev).next = node; }
        self.prev = node;
        node
    }
}


impl<T> Default for Node<T> {
    fn default() -> Self {
        Node::new()
    }
}


// List

pub struct List<T> {
    head: *mut Node<T>,
    tail: *mut Node<T>,
}


impl<T> List<T> {
    pub fn new() -> Self {
        let head: *mut Node<T> = malloc!(Default::default());
        let tail: *mut Node<T> = malloc!(Default::default());
        unsafe {
            (*head).next = tail;
            (*tail).prev = head;
        }
        List { head, tail }
    }

    pub fn push_back(&mut self, value: T) -> *mut Node<T> {
        unsafe {
            (*self.tail).insert_before(value)
        }
    }
}


impl<T> Drop for List<T> {
    fn drop(&mut self) {
        let mut p_node = self.head;
        while !p_node.is_null() {
            let node = unsafe { free!(p_node) };
            p_node = node.next;
        }
    }
}


impl<T> Default for List<T> {
    fn default() -> Self {
        List::new()
    }
}


// Iteration

pub struct Iter<'a, T> {
    node: &'a Node<T>,
}


impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let value = self.node.value.as_ref();
        self.node = unsafe { &*self.node.next };
        value
    }
}


impl<'a, T> IntoIterator for &'a List<T> {
    type Item = &'a T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        Iter { node: unsafe { &*(*self.head).next } }
    }
}


// Serialization

impl<T> Debug for List<T> where T: Debug {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self).finish()
    }
}


impl<T> Serialize for List<T> where T: Serialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut seq = serializer.serialize_seq(None)?;
        for value in self {
            seq.serialize_element(value)?;
        }
        seq.end()
    }
}
