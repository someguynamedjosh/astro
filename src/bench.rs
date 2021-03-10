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
        vec![
            derivation_with_ptrs_dyn!(root; *root.borrow()),
            derivation_with_ptrs_dyn!(root; *root.borrow()),
        ]
    };
    for depth in 1..100 {
        let mut new_values =
            vec![derivation_with_ptrs_dyn!(prev_start: last_values[0]; *prev_start.borrow())];
        for position in 0..depth {
            new_values.push(derivation_with_ptrs_dyn!(
                left: last_values[position],
                right: last_values[position + 1];
                *left.borrow() + *right.borrow()
            ));
        }
        new_values.push(derivation_with_ptrs_dyn!(
            prev_end: last_values[depth];
            *prev_end.borrow()
        ));
        last_values = new_values;
    }
    b.iter(move || {
        value = (value + 1.0) % 10.0;
        root.set(value);
        value
    });
}
