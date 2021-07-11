// Copyright (c) 2021, Andrey Dubovik <andrei@dubovik.eu>

// A generally inefficient algorithm to construct connected components dynamically
// (However, this algorithm is efficient if connected components are bounded.)

use std::collections::HashMap;


pub struct Partitioner<'a> {
    partitions: &'a mut HashMap<usize, Vec<usize>>,
    index: HashMap<usize, usize>,
}


impl<'a> Partitioner<'a> {
    pub fn new(partitions: &'a mut HashMap<usize, Vec<usize>>) -> Self {
        Partitioner {
            partitions,
            index: HashMap::new(),
        }
    }

    pub fn insert(&mut self, i: usize, j: usize) {
        if i == j {
            return;  //  drop self references
        }
        let pi = self.index.get(&i).cloned();
        let pj = self.index.get(&j).cloned();
        match (pi, pj) {
            (None, None) => {
                self.partitions.insert(i, vec![i, j]);
                self.index.insert(i, i);
                self.index.insert(j, i);
            },
            (None, Some(pj)) => {
                self.partitions.get_mut(&pj).unwrap().push(i);
                self.index.insert(i, pj);
            },
            (Some(pi), None) => {
                self.partitions.get_mut(&pi).unwrap().push(j);
                self.index.insert(j, pi);
            },
            (Some(pi), Some(pj)) => {
                if pi != pj {
                    let vi = &self.partitions[&pi];
                    let vj = &self.partitions[&pj];
                    if vj.len() < vi.len() {
                        for j in vj {
                            self.index.insert(*j, pi);
                        }
                        let mut vj = self.partitions.remove(&pj).unwrap();
                        self.partitions.get_mut(&pi).unwrap().append(&mut vj);
                    }
                    else {
                        for i in vi {
                            self.index.insert(*i, pj);
                        }
                        let mut vi = self.partitions.remove(&pi).unwrap();
                        self.partitions.get_mut(&pj).unwrap().append(&mut vi);
                    }
                }
            },
        }
    }
}
