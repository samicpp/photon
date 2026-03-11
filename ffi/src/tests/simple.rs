#![cfg(test)]

// use crate::httpcpp::{add, add_f64, add_test};


#[test]
fn four_is_four(){
    assert!(4 == 4);
}


// #[test]
// fn httpcpp_test(){
//     unsafe{
//         assert_eq!(add_f64(1.0, 2.0), 3.0);
//         assert_eq!(add(1, 2), 3);
//         assert_eq!(add_test(), 0);
//     }
// }