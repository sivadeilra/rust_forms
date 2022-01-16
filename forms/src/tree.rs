use std::cell::Cell;
use std::rc::Rc;
use std::rc::Weak;

pub struct Tree<T> {
    root: Rc<NodeData<T>>,
}

struct NodeData<T> {
    parent: Cell<Option<Weak<NodeData<T>>>>,
    data: T,
    children: Cell<Vec<Rc<NodeData<T>>>>,
}

pub struct Node<T>(Rc<NodeData<T>>);

impl<T> Tree<T> {
    pub fn new(root: T) -> Tree<T> {
        Tree {
            root: Rc::new(NodeData {
                data: root,
                parent: Cell::new(None),
                children: Cell::new(Vec::new()),
            }),
        }
    }
}

impl<T> Node<T> {
    pub fn get(&self) -> &T {
        &self.0.data
    }

    pub fn iter(&self) -> NodeIter<T> {
        NodeIter {
            pos: 0,
            node: Node(Rc::clone(&self.0)),
        }
    }

    pub fn add(&self, data: T) -> Node<T> {
        let new_node = Node(Rc::new(NodeData {
            data,
            parent: Cell::new(None),
            children: Cell::new(Vec::new()),
        }));

        let mut children = self.0.children.take();
        children.push(Rc::clone(&new_node.0));
        self.0.children.replace(children);

        new_node
    }

    /// Remove this node from the tree.
    /// Invalid on root node.
    pub fn remove(&self) {
        match self.0.parent.take() {
            None => {
                // No need to put the parent back.
            }
            Some(parent_weak) => {
                // Remember to put parent back.

                let parent_opt = parent_weak.upgrade();
                self.0.parent.replace(Some(parent_weak)); // put parent back.

                if let Some(parent) = parent_opt {
                    let mut children = parent.children.take();
                    // Look for self in this list of children.

                    for i in 0..children.len() {
                        let c = &children[i];
                        if is_same_rc(c, &self.0) {
                            children.remove(i);
                            break;
                        }
                    }

                    // Put children back.
                    parent.children.replace(children);
                } else {
                    // Weak link is dead. Tree is being torn down.
                }
            }
        }
    }
}

pub struct NodeIter<T> {
    pos: usize,
    node: Node<T>,
}

impl<T> Iterator for NodeIter<T> {
    type Item = Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let children = self.node.0.children.take();
        if self.pos < children.len() {
            let next = Node(Rc::clone(&children[self.pos]));
            self.node.0.children.replace(children);
            self.pos += 1;
            Some(next)
        } else {
            None
        }
    }
}

fn is_same_rc<T>(a: &Rc<T>, b: &Rc<T>) -> bool {
    let ra: &T = &*a;
    let rb: &T = &*b;
    ra as *const T == rb as *const T
}
