use std::collections::VecDeque;

/// The path through a forest of CST nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CstPath(usize, VecDeque<usize>);
impl CstPath {
    pub fn here_at(index: usize) -> Self {
        CstPath(index, VecDeque::new())
    }
    pub fn with_new_parent(mut self, at: usize) -> Self {
        self.1.push_front(self.0);
        self.0 = at;
        self
    }
    pub fn with_new_parent_mut(&mut self, at: usize) {
        self.1.push_front(self.0);
        self.0 = at;
    }
    pub fn shift_left_by(mut self, width: usize) -> Self {
        self.0 += width;
        self
    }
    pub fn shift_left_mut(&mut self, width: usize) {
        self.0 += width;
    }
}
