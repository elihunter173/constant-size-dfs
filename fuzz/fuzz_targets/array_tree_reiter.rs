#![no_main]

use constant_size_dfs::array_tree::Tree;
use libfuzzer_sys::fuzz_target;

fn fuzz<const N: usize>(data: &[u8]) {
    let (mut tree, expected) = Tree::<_, N>::arbitrary(data);
    let actual1: Vec<_> = tree.dfs_iter_mut().map(|v| *v).collect();
    let actual2: Vec<_> = tree.dfs_iter_mut().map(|v| *v).collect();
    assert_eq!(expected, actual1);
    assert_eq!(expected, actual2);
}

fuzz_target!(|data: &[u8]| {
    let Some((&size, data)) = data.split_first() else {
        return;
    };
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
