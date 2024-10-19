pub mod binary_tree;
pub mod tagged_ptr;

// let mut tree = Tree::new(NodePtr::new(
//     0,
//     NodePtr::new(1, NodePtr::leaf(2), NodePtr::null()),
//     NodePtr::new(3, NodePtr::leaf(4), NodePtr::leaf(5)),
// ));
//
// println!("before: {tree:#?}");
// for (i, v) in tree.dfs_iter_mut().enumerate() {
//     if i > 2 {
//         break;
//     }
//     println!("{v}");
// }
// println!("after: {tree:#?}");
