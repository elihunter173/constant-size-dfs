use std::{
    fmt::{self, Debug, Write},
    marker::PhantomData,
    ptr::{self},
};

#[derive(Debug)]
struct Tree<T: Debug> {
    root: NodePtr<T>,
}

impl<T: Debug> Tree<T> {
    fn new(root: NodePtr<T>) -> Self {
        Self { root }
    }

    fn dfs_iter_mut(&mut self) -> DfsIterMut<T> {
        DfsIterMut {
            prev: NodePtr::null(),
            cur: self.root,
            lifetime: PhantomData,
        }
    }
}

impl<T: Debug> Drop for Tree<T> {
    fn drop(&mut self) {
        let iter = LeafsFirst {
            prev: NodePtr::null(),
            cur: self.root,
            lifetime: PhantomData
        };
        for node in iter {
            unsafe { println!("dropping {:?}", (*node).val) };
            let _ = unsafe { Box::from_raw(node) };
        }
    }
}

#[derive(Debug)]
struct Node<T> {
    val: T,
    left: NodePtr<T>,
    right: NodePtr<T>,
}

// Should have assert that Node<T> is align > 2

const SEEN_BIT: usize = 1;

// Can be null
struct NodePtr<T>(*mut Node<T>);

impl<T: Debug> Debug for NodePtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = self.as_ptr();
        let flag = self.0 as usize & SEEN_BIT;
        write!(f, "<0x{:0x}|{}>", ptr as usize, flag)?;
        if let Some(node) = unsafe { ptr.as_ref() } {
            f.write_char(' ')?;
            node.fmt(f)?;
        }
        Ok(())
    }
}

impl<T> Clone for NodePtr<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for NodePtr<T> {}

impl<T> NodePtr<T> {
    fn null() -> Self {
        Self(ptr::null_mut())
    }

    fn new(val: T, left: NodePtr<T>, right: NodePtr<T>) -> Self {
        let ptr = Box::into_raw(Box::new(Node { val, left, right }));
        Self(ptr)
    }

    fn leaf(val: T) -> Self {
        Self::new(val, NodePtr::null(), NodePtr::null())
    }

    fn is_seen(self) -> bool {
        self.0 as usize & SEEN_BIT == SEEN_BIT
    }

    fn seen(self) -> Self {
        let addr = self.0 as usize | SEEN_BIT;
        Self(addr as _)
    }

    fn unseen(self) -> Self {
        let addr = self.0 as usize & !SEEN_BIT;
        Self(addr as _)
    }

    fn as_ptr(self) -> *mut Node<T> {
        let addr = self.0 as usize & !SEEN_BIT;
        addr as _
    }
}

// These Deref impls are confusing

struct DfsIterMut<'tree, T> {
    // Maybe I should just use *mut Node<T>
    prev: NodePtr<T>,
    cur: NodePtr<T>,
    lifetime: PhantomData<&'tree T>,
}

// Maybe I could simplify this by considering null nodes visited. Idk probably not

impl<'tree, T> Iterator for DfsIterMut<'tree, T> {
    type Item = &'tree mut T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // SAFETY: We're guarnteed the pointers live for the lifespan of 'tree
            let cur: &'tree mut Node<T> = unsafe { self.cur.as_ptr().as_mut()? };
            match (cur.left.is_seen(), cur.right.is_seen()) {
                // First time visiting, yield this node
                (false, false) => {
                    let old_left = cur.left;
                    cur.left = self.prev.seen();
                    if old_left.as_ptr().is_null() {
                        // Pretend like we've just finished the left
                        self.prev = old_left;
                    } else {
                        self.cur = old_left;
                        self.prev = NodePtr(cur);
                    }
                    return Some(&mut cur.val);
                }
                // Second time visiting, just go to the right
                (true, false) => {
                    let old_right = cur.right;
                    cur.right = self.prev.seen();
                    if old_right.as_ptr().is_null() {
                        // Pretend like we've just finished the right
                        self.prev = old_right;
                    } else {
                        self.cur = old_right;
                        self.prev = NodePtr(cur);
                    }
                }
                // Invalid state. Tho theoretically we could visit the left
                (false, true) => unreachable!("we always visit left before right"),
                // Visited this whole subtree, re-construct things and go up
                (true, true) => {
                    let real_right = self.prev;
                    let real_left = cur.right;
                    let parent = cur.left;
                    cur.left = real_left.unseen();
                    cur.right = real_right.unseen();
                    self.cur = parent;
                    self.prev = NodePtr(cur);
                }
            }
        }
    }
}

// NOTE: It's okay if this doesn't run. The tree will leak some nodes but be
// safe
impl<'tree, T> Drop for DfsIterMut<'tree, T> {
    fn drop(&mut self) {
        println!("dropping iter");
    }
}

struct LeafsFirst<'tree, T> {
    // Maybe I should just use *mut Node<T>
    prev: NodePtr<T>,
    cur: NodePtr<T>,
    lifetime: PhantomData<&'tree T>,
}

impl<'tree, T> Iterator for LeafsFirst<'tree, T> {
    type Item = *mut Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // SAFETY: We're guarnteed the pointers live for the lifespan of 'tree
            let cur: &'tree mut Node<T> = unsafe { self.cur.as_ptr().as_mut()? };
            match (cur.left.is_seen(), cur.right.is_seen()) {
                // First time visiting, yield this node
                (false, false) => {
                    let old_left = cur.left;
                    cur.left = self.prev.seen();
                    if old_left.as_ptr().is_null() {
                        // Pretend like we've just finished the left
                        self.prev = old_left;
                    } else {
                        self.cur = old_left;
                        self.prev = NodePtr(cur);
                    }
                }
                // Second time visiting, just go to the right
                (true, false) => {
                    let old_right = cur.right;
                    cur.right = self.prev.seen();
                    if old_right.as_ptr().is_null() {
                        // Pretend like we've just finished the right
                        self.prev = old_right;
                    } else {
                        self.cur = old_right;
                        self.prev = NodePtr(cur);
                    }
                }
                // Invalid state. Tho theoretically we could visit the left
                (false, true) => unreachable!("we always visit left before right"),
                // Visited this whole subtree, re-construct things and go up
                (true, true) => {
                    let real_right = self.prev;
                    let real_left = cur.right;
                    let parent = cur.left;
                    cur.left = real_left.unseen();
                    cur.right = real_right.unseen();
                    self.cur = parent;
                    self.prev = NodePtr(cur);
                    return Some(cur);
                }
            }
        }
    }
}

fn main() {
    let mut tree = Tree::new(NodePtr::new(
        0,
        NodePtr::new(1, NodePtr::leaf(2), NodePtr::null()),
        NodePtr::new(3, NodePtr::leaf(4), NodePtr::leaf(5)),
    ));

    println!("before: {tree:#?}");
    for (i, v) in tree.dfs_iter_mut().enumerate() {
        if i > 2 {
            break;
        }
        println!("{v}");
    }
    println!("after: {tree:#?}");
}

// TODO: dropping iter needs to fixup the tree
