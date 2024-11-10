use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ptr::{self},
};

use crate::tagged_ptr::TaggedPtr;

pub struct Tree<T, const N: usize> {
    root: *mut Node<T, N>,
}

impl<T: Debug, const N: usize> Debug for Tree<T, N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(&format!("Tree<_, {N}>"))
            .field("root", &TaggedPtr::from_untagged(self.root))
            .finish()
    }
}

#[derive(Debug)]
pub struct Node<T, const N: usize> {
    val: T,
    children: [TaggedPtr<Node<T, N>>; N],
}

impl<T, const N: usize> Tree<T, N> {
    pub fn new(root: Option<Box<Node<T, N>>>) -> Self {
        Self { root: to_ptr(root) }
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

type Fence = u16;

impl<const N: usize> Tree<u8, N> {
    pub fn arbitrary(data: &[u8]) -> (Self, Vec<u8>) {
        assert!(data.len() < Fence::MAX as usize, "Fence size is too small");
        let mut values = Vec::with_capacity(data.len());
        let root = node_from_arbitrary(data, &mut values);
        let tree = Self::new(root);
        (tree, values)
    }
}

fn node_from_arbitrary<const N: usize>(
    data: &[u8],
    values: &mut Vec<u8>,
) -> Option<Box<Node<u8, N>>> {
    let (&val, data) = data.split_first()?;
    values.push(val);

    let mut children = [const { None }; N];
    let num_mid_fences = N - 1;
    let Some((mid_fences, data)) = data.split_at_checked(num_mid_fences * size_of::<Fence>())
    else {
        return Some(Node::alloc(val, children));
    };

    let mut fences = [0; N];
    *fences.last_mut().unwrap() = data.len();
    for (i, v) in mid_fences.chunks(size_of::<Fence>()).enumerate() {
        let v = Fence::from_ne_bytes(v.try_into().unwrap());
        fences[i + 1] = v as usize % (data.len() + 1);
    }
    fences.sort();

    for (i, slot) in children.iter_mut().enumerate() {
        let range = fences[i]..fences.get(i + 1).copied().unwrap_or(data.len());
        *slot = node_from_arbitrary(&data[range], values);
    }
    Some(Node::alloc(val, children))
}

fn to_ptr<T>(node: Option<Box<T>>) -> *mut T {
    node.map(|n| Box::leak(n) as *mut _)
        .unwrap_or(ptr::null_mut())
}

impl<T, const N: usize> Node<T, N> {
    pub fn alloc(val: T, children: [Option<Box<Node<T, N>>>; N]) -> Box<Node<T, N>> {
        let mut converted = [TaggedPtr::from_untagged(ptr::null_mut()); N];
        for (slot, node) in converted.iter_mut().zip(children) {
            *slot = TaggedPtr::from_untagged(to_ptr(node));
        }
        Box::new(Node {
            val,
            children: converted,
        })
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
        mut tree: Tree<T, N>,
    ) {
        let expected: Vec<T> = expected.into_iter().collect();
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

    fn tree<T, const N: usize>(root: TaggedPtr<Node<T, N>>) -> Tree<T, N> {
        Tree {
            root: root.as_untagged(),
        }
    }

    #[test]
    fn empty() {
        assert_dfs_valid::<i32, 2>([], tree(null()));
    }

    #[test]
    fn one() {
        assert_dfs_valid::<_, 2>([0], tree(leaf(0)));
    }

    #[test]
    fn two() {
        assert_dfs_valid([0, 1], tree(node(0, [leaf(1), null()])));
    }

    #[test]
    fn basic() {
        assert_dfs_valid(
            0..=5,
            tree(node(
                0,
                [node(1, [leaf(2), null()]), node(3, [leaf(4), leaf(5)])],
            )),
        );
    }

    #[test]
    fn nochildren() {
        assert_dfs_valid(["hi"], tree(node("hi", [])));
    }

    #[test]
    fn linked_list() {
        let list = node(0, [node(1, [node(2, [leaf(3)])])]);
        assert_dfs_valid(0..=3, tree(list));
    }

    #[test]
    fn iter_fixes_tree() {
        let mut tree = tree(node(
            0,
            [node(1, [leaf(2), null()]), node(3, [leaf(4), leaf(5)])],
        ));
        let mut iter = tree.dfs_iter_mut();
        assert_eq!(Some(&mut 0), iter.next());
        assert_eq!(Some(&mut 1), iter.next());
        assert_eq!(Some(&mut 2), iter.next());
        drop(iter);
        assert_dfs_valid(0..=5, tree);
    }
}
