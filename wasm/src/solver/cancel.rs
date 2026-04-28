// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Cooperative cancellation flag for the solver.

#[cfg(not(target_arch = "wasm32"))]
pub use native::CancelFlag;

#[cfg(target_arch = "wasm32")]
pub use wasm::CancelFlag;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::sync::atomic::{AtomicI32, Ordering};

    /// Wraps a borrowed `&AtomicI32`.
    pub struct CancelFlag<'a>(&'a AtomicI32);

    impl<'a> CancelFlag<'a> {
        /// Wraps an existing `AtomicI32` for the solver to observe.
        pub fn new(flag: &'a AtomicI32) -> Self {
            Self(flag)
        }

        /// Returns `true` if any thread has signaled cancellation.
        #[inline]
        pub(crate) fn is_cancelled(&self) -> bool {
            self.0.load(Ordering::Relaxed) != 0
        }
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::marker::PhantomData;

    use js_sys::{Int32Array, SharedArrayBuffer};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        /// Calls JS's `Atomics.load(array, index)`.
        #[wasm_bindgen(js_namespace = Atomics, js_name = load)]
        fn atomics_load(array: &Int32Array, index: u32) -> i32;
    }

    /// Wraps an `Int32Array` view over a `SharedArrayBuffer`.
    pub struct CancelFlag<'a> {
        view: Int32Array,
        _phantom: PhantomData<&'a ()>,
    }

    impl<'a> CancelFlag<'a> {
        /// Wraps a `SharedArrayBuffer` as a cancel flag.
        pub fn from_sab(sab: &SharedArrayBuffer) -> Self {
            Self {
                view: Int32Array::new(sab.as_ref()),
                _phantom: PhantomData
            }
        }

        /// Returns `true` if the main thread has signaled cancellation.
        #[inline]
        pub(crate) fn is_cancelled(&self) -> bool {
            atomics_load(&self.view, 0) != 0
        }
    }
}