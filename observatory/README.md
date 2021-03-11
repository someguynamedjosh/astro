# Observatory

[![Latest version on crates.io](https://img.shields.io/crates/v/observatory)](https://crates.io/crates/observatory)

Provides MobX style observables. Example:

```rust
use observatory as o;
o::init();
let first_name = o::observable("William");
let last_name = o::observable("Riker");
let nickname = o::observable::<Option<&'static str>>(None);
// A derivation is run the first time it is created, and the guts of the Derivation type will
// detect that the function borrows nickname, first_name, and last_name during that time.
let display_name = o::derivation_with_ptrs!(
    first_name, last_name, nickname;
    if let Some(name) = *nickname.borrow() {
        format!("{}", name)
    } else {
        format!("{} {}", *first_name.borrow(), *last_name.borrow())
    }
);
// Prints "William Riker"
let logger = o::derivation_with_ptrs!(
    display_name;
    println!("{}", *display_name.borrow())
);
// Prints "Will of Yam Riker"
first_name.set("Will of Yam");
// Prints "Number One"
// After executing this function the library will detect that `display_name` didn't need to
// borrow first_name or last_name to update its value.
nickname.set(Some("Number One"));
// Causes no updates, display_name has automatically unsubscribed from updates to last_name.
last_name.set("Something else");
```
