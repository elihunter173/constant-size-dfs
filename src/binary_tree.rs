use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ptr::{self},
};

use crate::tagged_ptr::TaggedPtr;

pub struct Tree<T> {
    root: *mut Node<T>,
}

impl<T: Debug> Debug for Tree<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tree")
            .field("root", &TaggedPtr::from_untagged(self.root))
            .finish()
    }
}

#[derive(Debug)]
pub struct Node<T> {
    val: T,
    left: TaggedPtr<Node<T>>,
    right: TaggedPtr<Node<T>>,
}

impl<T> Tree<T> {
    pub fn new(root: *mut Node<T>) -> Self {
        Self { root }
    }

    pub fn dfs_iter_mut(&mut self) -> DfsIterMut<T> {
        let iter = NodeIter {
            prev: ptr::null_mut(),
            cur: self.root,
            lifetime: PhantomData,
        };
        DfsIterMut { iter }
    }
}

impl<T> Drop for Tree<T> {
    fn drop(&mut self) {
        // We want to visit the leaves first
        let iter = NodeIter::<T, 2> {
            prev: ptr::null_mut(),
            cur: self.root,
            lifetime: PhantomData,
        };
        for node in iter {
            let _ = unsafe { Box::from_raw(node) };
        }
    }
}

pub struct NodeIter<'tree, T, const RETURN_ON_VISIT: usize> {
    prev: *mut Node<T>,
    cur: *mut Node<T>,
    lifetime: PhantomData<&'tree T>,
}

// NOTE: It's okay if this doesn't run. The tree will leak some nodes but be
// safe
impl<'tree, T, const RETURN_ON_VISIT: usize> Drop for NodeIter<'tree, T, RETURN_ON_VISIT> {
    fn drop(&mut self) {
        println!("dropping iter");
        // TODO: dropping iter needs to fixup the tree
    }
}

impl<'tree, T, const RETURN_ON_VISIT: usize> Iterator for NodeIter<'tree, T, RETURN_ON_VISIT> {
    type Item = *mut Node<T>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // SAFETY: We're guarnteed the pointers live for the lifespan of 'tree
            let cur: &'tree mut Node<T> = unsafe { self.cur.as_mut()? };
            match (cur.left.is_seen(), cur.right.is_seen()) {
                // First time visiting, yield this node
                (false, false) => {
                    let old_left = cur.left.as_untagged();
                    cur.left = TaggedPtr::from_untagged(self.prev).seen();
                    if old_left.is_null() {
                        // Pretend like we've just finished the left
                        self.prev = old_left;
                    } else {
                        self.cur = old_left;
                        self.prev = cur;
                    }
                    if RETURN_ON_VISIT == 0 {
                        return Some(cur);
                    }
                }
                // Second time visiting, just go to the right
                (true, false) => {
                    let old_right = cur.right.as_untagged();
                    cur.right = TaggedPtr::from_untagged(self.prev).seen();
                    if old_right.is_null() {
                        // Pretend like we've just finished the right
                        self.prev = old_right;
                    } else {
                        self.cur = old_right;
                        self.prev = cur;
                    }
                    if RETURN_ON_VISIT == 1 {
                        return Some(cur);
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
                    cur.right = TaggedPtr::from_untagged(real_right).unseen();
                    self.cur = parent.as_untagged();
                    self.prev = cur;
                    if RETURN_ON_VISIT == 2 {
                        return Some(cur);
                    }
                }
            }
        }
    }
}

pub struct DfsIterMut<'tree, T> {
    iter: NodeIter<'tree, T, 0>,
}

impl<'tree, T> Iterator for DfsIterMut<'tree, T> {
    type Item = &'tree mut T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter
            .next()
            .map(|node| unsafe { &mut node.as_mut().expect("should not be null").val })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_dfs_valid<T: Clone + Debug + PartialEq>(
        expected: impl IntoIterator<Item = T>,
        root: TaggedPtr<Node<T>>,
    ) {
        let expected: Vec<T> = expected.into_iter().collect();
        let mut tree = Tree::new(root.as_untagged());
        let actual: Vec<T> = tree.dfs_iter_mut().map(|v| v.clone()).collect();
        assert_eq!(expected, actual);
    }

    fn null<T>() -> TaggedPtr<Node<T>> {
        TaggedPtr::from_untagged(ptr::null_mut())
    }

    fn node<T>(val: T, left: TaggedPtr<Node<T>>, right: TaggedPtr<Node<T>>) -> TaggedPtr<Node<T>> {
        let node = Node { val, left, right };
        TaggedPtr::from_untagged(Box::into_raw(Box::new(node)))
    }

    fn leaf<T>(val: T) -> TaggedPtr<Node<T>> {
        node(val, null(), null())
    }

    #[test]
    fn empty() {
        assert_dfs_valid::<i32>([], null());
    }

    #[test]
    fn one() {
        assert_dfs_valid([0], leaf(0));
    }

    #[test]
    fn two() {
        assert_dfs_valid([0, 1], node(0, leaf(1), null()));
    }

    #[test]
    fn basic() {
        assert_dfs_valid(
            0..=5,
            node(0, node(1, leaf(2), null()), node(3, leaf(4), leaf(5))),
        );
    }
}
