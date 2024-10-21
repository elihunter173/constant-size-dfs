use std::{
    fmt::Debug,
    marker::PhantomData,
    ptr::{self},
};

use crate::tagged_ptr::TaggedPtr;

#[derive(Debug)]
pub struct Tree<T, const N: usize> {
    root: *mut Node<T, N>,
}

#[derive(Debug)]
pub struct Node<T, const N: usize> {
    val: T,
    children: [TaggedPtr<Node<T, N>>; N],
}

impl<T, const N: usize> Tree<T, N> {
    pub fn new(root: *mut Node<T, N>) -> Self {
        Self { root }
    }

    pub fn dfs_iter_mut(&mut self) -> DfsIterMut<T, N> {
        let iter = NodeIter {
            prev: ptr::null_mut(),
            cur: self.root,
            lifetime: PhantomData,
        };
        DfsIterMut { iter }
    }
}

impl<T, const N: usize> Drop for Tree<T, N> {
    fn drop(&mut self) {
        // We want to visit the leaves first
        let iter = NodeIter::<T, N, N> {
            prev: ptr::null_mut(),
            cur: self.root,
            lifetime: PhantomData,
        };
        for node in iter {
            let _ = unsafe { Box::from_raw(node) };
        }
    }
}

pub struct NodeIter<'tree, T, const N: usize, const RETURN_ON_VISIT: usize> {
    prev: *mut Node<T, N>,
    cur: *mut Node<T, N>,
    lifetime: PhantomData<&'tree T>,
}

// NOTE: It's okay if this doesn't run. The tree will leak some nodes but be
// safe
impl<'tree, T, const N: usize, const RETURN_ON_VISIT: usize> Drop
    for NodeIter<'tree, T, N, RETURN_ON_VISIT>
{
    fn drop(&mut self) {
        // Ascend the tree until we reach the top (i.e. null self.cur) and
        while let Some(cur) = unsafe { self.cur.as_mut() } {
            let first_unvisited = cur
                .children
                .iter()
                .position(|node_ptr| !node_ptr.is_seen())
                .unwrap_or(N);

            if first_unvisited == 0 {
                // If we haven't visited any children, then our previous node is our parent
                self.cur = self.prev;
                self.prev = cur;
            } else {
                let parent = cur.children[0];
                for i in 0..(first_unvisited - 1) {
                    cur.children[i] = cur.children[i + 1].unseen();
                }
                cur.children[first_unvisited - 1] = TaggedPtr::from_untagged(self.prev).unseen();
                self.cur = parent.as_untagged();
                self.prev = cur;
            }
        }
    }
}

impl<'tree, T, const N: usize, const RETURN_ON_VISIT: usize> Iterator
    for NodeIter<'tree, T, N, RETURN_ON_VISIT>
{
    type Item = *mut Node<T, N>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // SAFETY: We're guarnteed the pointers live for the lifespan of 'tree
            let cur: &'tree mut Node<T, N> = unsafe { self.cur.as_mut()? };

            let first_unvisited = cur
                .children
                .iter()
                .position(|node_ptr| !node_ptr.is_seen())
                .unwrap_or(N);
            if first_unvisited < N {
                // Visit that child
                let child_to_visit = cur.children[first_unvisited].as_untagged();
                cur.children[first_unvisited] = TaggedPtr::from_untagged(self.prev).seen();
                if child_to_visit.is_null() {
                    // Return like we just visited this node
                    self.prev = child_to_visit;
                } else {
                    self.cur = child_to_visit;
                    self.prev = cur;
                }
            } else {
                // Visited all children, go re-construct things and go up.
                if first_unvisited == 0 {
                    // If we haven't visited any children, then our previous node is our parent
                    self.cur = self.prev;
                    self.prev = cur;
                } else {
                    let parent = cur.children[0];
                    for i in 0..(first_unvisited - 1) {
                        cur.children[i] = cur.children[i + 1].unseen();
                    }
                    cur.children[first_unvisited - 1] =
                        TaggedPtr::from_untagged(self.prev).unseen();
                    self.cur = parent.as_untagged();
                    self.prev = cur;
                }
            }

            if first_unvisited == RETURN_ON_VISIT {
                return Some(cur);
            }
        }
    }
}

pub struct DfsIterMut<'tree, T, const N: usize> {
    iter: NodeIter<'tree, T, N, 0>,
}

impl<'tree, T, const N: usize> Iterator for DfsIterMut<'tree, T, N> {
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

    fn assert_dfs_valid<T: Clone + Debug + PartialEq, const N: usize>(
        expected: impl IntoIterator<Item = T>,
        root: TaggedPtr<Node<T, N>>,
    ) {
        let expected: Vec<T> = expected.into_iter().collect();
        let mut tree = Tree::new(root.as_untagged());
        let actual: Vec<T> = tree.dfs_iter_mut().map(|v| v.clone()).collect();
        assert_eq!(expected, actual);
    }

    fn null<T, const N: usize>() -> TaggedPtr<Node<T, N>> {
        TaggedPtr::from_untagged(ptr::null_mut())
    }

    fn node<T, const N: usize>(
        val: T,
        children: [TaggedPtr<Node<T, N>>; N],
    ) -> TaggedPtr<Node<T, N>> {
        let node = Node { val, children };
        TaggedPtr::from_untagged(Box::into_raw(Box::new(node)))
    }

    fn leaf<T, const N: usize>(val: T) -> TaggedPtr<Node<T, N>> {
        node(val, [null(); N])
    }

    #[test]
    fn empty() {
        assert_dfs_valid::<i32, 2>([], null());
    }

    #[test]
    fn one() {
        assert_dfs_valid::<_, 2>([0], leaf(0));
    }

    #[test]
    fn two() {
        assert_dfs_valid([0, 1], node(0, [leaf(1), null()]));
    }

    #[test]
    fn basic() {
        assert_dfs_valid(
            0..=5,
            node(0, [node(1, [leaf(2), null()]), node(3, [leaf(4), leaf(5)])]),
        );
    }

    #[test]
    fn nochildren() {
        assert_dfs_valid(["hi"], node("hi", []))
    }

    #[test]
    fn linked_list() {
        let list = node(0, [node(1, [node(2, [leaf(3)])])]);
        assert_dfs_valid(0..=3, list);
    }
}
