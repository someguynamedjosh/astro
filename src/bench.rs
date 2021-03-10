#![cfg(test)]
#![cfg(not(miri))]

extern crate test;
use test::Bencher;

#[bench]
fn bench_large_network(b: &mut Bencher) {
    use crate::*;
    if !is_initialized() {
        init();
    }
    let mut value = 0f64;
    let root = ObservablePtr::new(value);
    let mut last_values = {
        ptr_clone!(root);
        let root2 = root.clone();
        vec![
            DerivationPtr::new_boxed(move || *root.borrow()),
            DerivationPtr::new_boxed(move || *root2.borrow()),
        ]
    };
    for depth in 1..100 {
        let prev_start = Clone::clone(&last_values[0]);
        let mut new_values = vec![DerivationPtr::new_boxed(move || *prev_start.borrow())];
        for position in 0..depth {
            let left = Clone::clone(&last_values[position]);
            let right = Clone::clone(&last_values[position + 1]);
            new_values.push(DerivationPtr::new_boxed(move || {
                *left.borrow() + *right.borrow()
            }));
        }
        let prev_end = Clone::clone(&last_values[depth]);
        new_values.push(DerivationPtr::new_boxed(move || *prev_end.borrow()));
        last_values = new_values;
    }
    b.iter(move || {
        value = (value + 1.0) % 10.0;
        root.set(value);
        value
    });
}
