// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 CloudHolic

//! Machine-learning support for the solver.

use cfg_if::cfg_if;

cfg_if! {
    if #[cfg(feature = "tracing")] {
        pub mod board;
        pub mod instance;
        pub mod polyomino;
        pub mod tracer;
        
        pub(crate) mod canonical;
        pub(crate) mod piece_pool;
        pub(crate) mod target;
    }
}

cfg_if! {
    if #[cfg(feature = "tracing")] {
        pub use board::UnionBoard;
        pub use instance::build_instance;
        pub use polyomino::PolyominoCatalog;
        pub use tracer::Tracer;
        
        pub(crate) use board::{Group, GroupId};
        pub(crate) use canonical::{BITMAP_SIZE, canonical_bitmap};
        pub(crate) use piece_pool::build_piece_pool;
        pub(crate) use target::build_target_cells;
        pub(crate) use tracer::BranchEvent;
        
        pub(crate) const GRID_COLS: u16 = 22;
    }
}