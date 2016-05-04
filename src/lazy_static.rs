/**
A macro for declaring lazily evaluated statics.
Using this macro, it is possible to have `static`s that require code to be
executed at runtime in order to be initialized.

Forked from https://github.com/rust-lang-nursery/lazy-static.rs

This version uses atomics instead of `std::sync::Once`, so it works also for `#![no_std]` builds.
It requires a nightly Rust version.

# Syntax
```text
lazy_static! {
    [pub] static ref NAME_1: TYPE_1 = EXPR_1;
    [pub] static ref NAME_2: TYPE_2 = EXPR_2;
    ...
    [pub] static ref NAME_N: TYPE_N = EXPR_N;
}
```
Metadata (such as doc comments) is allowed on each ref.
# Semantic
For a given `static ref NAME: TYPE = EXPR;`, the macro generates a unique type that
implements `Deref<TYPE>` and stores it in a static with name `NAME`. (Metadata ends up
attaching to this type.)
On first deref, `EXPR` gets evaluated and stored internally, such that all further derefs
can return a reference to the same object.
Like regular `static mut`s, this macro only works for types that fulfill the `Sync`
trait.
# Example
Using the macro:

```rust
#![feature(const_fn)]

#[macro_use]
extern crate once;
use std::collections::HashMap;
lazy_static! {
    static ref HASHMAP: HashMap<u32, &'static str> = {
        let mut m = HashMap::new();
        m.insert(0, "foo");
        m.insert(1, "bar");
        m.insert(2, "baz");
        m
    };
    static ref COUNT: usize = HASHMAP.len();
    static ref NUMBER: u32 = times_two(21);
}
fn times_two(n: u32) -> u32 { n * 2 }
fn main() {
    println!("The map has {} entries.", *COUNT);
    println!("The entry for `0` is \"{}\".", HASHMAP.get(&0).unwrap());
    println!("A expensive calculation on a static results in: {}.", *NUMBER);
}
```
# Implementation details
The `Deref` implementation uses a hidden static variable that is guarded by a atomic check on each access.
**/
#[macro_export]
macro_rules! lazy_static {
    ($(#[$attr:meta])* static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static!(PRIV, $(#[$attr])* static ref $N : $T = $e; $($t)*);
    };
    ($(#[$attr:meta])* pub static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static!(PUB, $(#[$attr])* static ref $N : $T = $e; $($t)*);
    };
    ($VIS:ident, $(#[$attr:meta])* static ref $N:ident : $T:ty = $e:expr; $($t:tt)*) => {
        lazy_static!(MAKE TY, $VIS, $(#[$attr])*, $N);
        impl $crate::__core::ops::Deref for $N {
            type Target = $T;
            fn deref<'a>(&'a self) -> &'a $T {
                #[inline(always)]
                fn __static_ref_initialize() -> $T { $e }

                unsafe {
                    use $crate::__core::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

                    #[inline(always)]
                    fn require_sync<T: Sync>(_: &T) { }

                    #[inline(always)]
                    unsafe fn __stability() -> &'static $T {
                        use $crate::__core::cell::UnsafeCell;

                        struct SyncCell(UnsafeCell<Option<$T>>);
                        unsafe impl Sync for SyncCell {}

                        static DATA: SyncCell = SyncCell(UnsafeCell::new(None));
                        static INITIALIZED: AtomicUsize = ATOMIC_USIZE_INIT;

                        if INITIALIZED.load(Ordering::Relaxed) != 2 {
                            // not initialized yet

                            // set state to `initializing`
                            match INITIALIZED.compare_and_swap(0, 1, Ordering::Relaxed) {
                                0 => {
                                    // initialize
                                    *DATA.0.get() = Some(__static_ref_initialize());
                                    // set state to `initialized`
                                    INITIALIZED.store(2, Ordering::Relaxed);
                                },
                                1 => {
                                    // currently initializing in another thread, keep spinning
                                    while INITIALIZED.load(Ordering::Relaxed) != 2 {}
                                },
                                2 => {}, // now it's initialized
                                _ => unreachable!(),
                            }
                        }

                        match *DATA.0.get() {
                            Some(ref x) => x,
                            None => unreachable!(),
                        }
                    }

                    let static_ref = __stability();
                    require_sync(static_ref);
                    static_ref
                }
            }
        }
        lazy_static!($($t)*);
    };
    (MAKE TY, PUB, $(#[$attr:meta])*, $N:ident) => {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        #[allow(missing_debug_implementations)]
        $(#[$attr])*
        pub struct $N {__private_field: ()}
        #[doc(hidden)]
        pub static $N: $N = $N {__private_field: ()};
    };
    (MAKE TY, PRIV, $(#[$attr:meta])*, $N:ident) => {
        #[allow(missing_copy_implementations)]
        #[allow(non_camel_case_types)]
        #[allow(dead_code)]
        #[allow(missing_debug_implementations)]
        $(#[$attr])*
        struct $N {__private_field: ()}
        #[doc(hidden)]
        static $N: $N = $N {__private_field: ()};
    };
    () => ()
}
