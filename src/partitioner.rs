// Copyright (c) 2019-2020, Andrey Dubovik <andrei@dubovik.eu>

// A generally inefficient algorithm to construct connected components dynamically
// (However, this algorithm is efficient if connected components are bounded.)

use std::ptr;

use crate::list::{List, Node};


pub struct Partitioner<'a> {
    list: &'a mut List<Vec<usize>>,
    nodes: Vec<*mut Node<Vec<usize>>>,
}


impl<'a> Partitioner<'a> {
    pub fn new(list: &'a mut List<Vec<usize>>) -> Self {
        Partitioner {
            list,
            nodes: Vec::new(),
        }
    }

    pub fn insert(&mut self, i: usize, j: usize) {
        if i == j {
            return;  //  drop self references
        }
        let len = i.max(j) + 1;
        if len > self.nodes.len() {
            self.nodes.resize_with(len, ptr::null_mut);
        }
        let pi = self.nodes[i];
        let pj = self.nodes[j];
        match (!pi.is_null(), !pj.is_null()) {
            (false, false) => {
                let node = self.list.push_back(vec![i, j]);
                self.nodes[i] = node;
                self.nodes[j] = node;
            },
            (false, true) => {
                unsafe { (*pj).as_mut().push(i); }
                self.nodes[i] = pj;
            },
            (true, false) => {
                unsafe { (*pi).as_mut().push(j); }
                self.nodes[j] = pi;
            },
            (true, true) => {
                if pi != pj {
                    let vi = unsafe { (*pi).as_mut() };
                    let vj = unsafe { (*pj).as_mut() };
                    if vj.len() < vi.len() {
                        for j in vj {
                            self.nodes[*j] = pi;
                        }
                        unsafe { vi.append(&mut (*pj).take()); }
                    }
                    else {
                        for i in vi {
                            self.nodes[*i] = pj;
                        }
                        unsafe { vj.append(&mut (*pi).take()); }
                    }
                }
            },
        }
    }
}
