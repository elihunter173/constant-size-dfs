#![no_main]

use constant_size_dfs::array_tree::Tree;
use libfuzzer_sys::fuzz_target;

fn fuzz<const N: usize>(data: &[u8]) {
    let Some((halt_after, data)) = data.split_at_checked(size_of::<u16>()) else {
        return;
    };
    let halt_after = u16::from_ne_bytes(halt_after.try_into().unwrap()) as usize;

    let (mut tree, expected) = Tree::<_, N>::arbitrary(data);
    let halt_after = halt_after.checked_rem(expected.len()).unwrap_or(0);

    let halted_at = tree.dfs_iter_mut().nth(halt_after).copied();
    let actual: Vec<_> = tree.dfs_iter_mut().map(|v| *v).collect();
    assert_eq!(expected.get(halt_after).copied(), halted_at);
    assert_eq!(expected, actual);
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
