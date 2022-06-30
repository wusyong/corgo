use std::mem::size_of;

use corgo::Runtime;

fn main() {
    let runtime = Runtime::new();
    let xxx = runtime.atom("");
    dbg!(size_of::<corgo::AtomIndex>());
}
