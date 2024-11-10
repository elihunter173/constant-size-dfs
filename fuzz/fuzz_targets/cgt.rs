#![no_main]

use constant_size_dfs::const_generic_tree::{Node, Tree};
use libfuzzer_sys::fuzz_target;

type Fence = u16;

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

fn fuzz<const N: usize>(data: &[u8]) {
    let mut expected = Vec::with_capacity(data.len());
    let root = node_from_arbitrary::<N>(data, &mut expected);
    let mut tree = Tree::new(root);
    let actual: Vec<_> = tree.dfs_iter_mut().map(|v| *v).collect();
    assert_eq!(actual, expected);
}

fuzz_target!(|data: &[u8]| {
    let Some((&size, data)) = data.split_first() else {
        return;
    };
    assert!(data.len() < Fence::MAX as usize, "Fence size is too small");
    // min size = 2, max size = 16 (inclusive)
    const MIN_SIZE: usize = 2;
    const MAX_SIZE: usize = 8;
    let size = (size as usize % (MAX_SIZE - MIN_SIZE + 1)) + MIN_SIZE;
    match size {
        2 => fuzz::<2>(data),
        3 => fuzz::<3>(data),
        4 => fuzz::<4>(data),
        5 => fuzz::<5>(data),
        6 => fuzz::<6>(data),
        7 => fuzz::<7>(data),
        8 => fuzz::<8>(data),
        _ => unreachable!(),
    }
});
