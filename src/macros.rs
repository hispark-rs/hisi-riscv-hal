//! Utility macros for the WS63 HAL.

// ── Stable/unstable gating (crate-local, esp-hal pattern) ───────────────────
// `#[instability::unstable]` cannot be applied to inline `pub mod foo;` declarations
// (rust-lang/rust#54727 — only the semicolon/external-file form is affected; inline
// mods WITH bodies work). These macros wrap a `pub mod foo;` declaration so the
// module is `pub` when `unstable` is on, `pub(crate)` when off (soft-gate) — keeping
// the module compiling in-crate (so a missed stable→unstable reference stays valid)
// while hiding it from external consumers without the feature. NO `#[macro_export]`
// — these are crate-internal helpers (esp-hal's are crate-private too, brought into
// scope via `pub(crate) use` and invoked as `crate::unstable_module!`). The
// `$(#[$meta])*` forwarding (incl. `#[path = "..."]`) is emitted on BOTH cfg branches
// so a `#[path]`-aliased module resolves on whichever copy survives.

/// Soft-gate a `pub mod foo;` declaration behind the `unstable` feature:
/// `pub mod foo;` when on, `pub(crate) mod foo;` when off. Use for modules the
/// crate's own stable code may reference (so the reference stays compiling as
/// `pub(crate)`). Both branches get `#[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]`
/// so docs.rs marks the module "requires unstable".
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! unstable_module {
    ($(
        $(#[$meta:meta])*
        pub mod $module:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[cfg(feature = "unstable")]
            #[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
            pub mod $module;

            $(#[$meta])*
            #[cfg(not(feature = "unstable"))]
            #[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
            #[allow(unused)]
            pub(crate) mod $module;
        )*
    };
}

/// Hard-gate a `pub mod foo;` declaration behind the `unstable` feature: the module
/// is `pub` when on, **absent** when off (no `pub(crate)` fallback — the module is
/// not compiled at all). Use for standalone drivers that nothing stable depends on
/// (saves flash + guarantees no stable code reaches them).
#[doc(hidden)]
#[allow(unused_macros)]
macro_rules! unstable_driver {
    ($(
        $(#[$meta:meta])*
        pub mod $module:ident;
    )*) => {
        $(
            $(#[$meta])*
            #[cfg(feature = "unstable")]
            #[cfg_attr(docsrs, doc(cfg(feature = "unstable")))]
            pub mod $module;
        )*
    };
}

// The macros are made crate-visible by `#[macro_use]` on `mod macros;` in lib.rs —
// invoke them by bare name (`unstable_module! { ... }`) from any submodule.

/// Create a type-erased enum for a peripheral type.
/// Example:
/// ```ignore
/// any_peripheral! {
///     pub peripheral AnySpi<'d> {
///         Spi0(crate::peripherals::Spi0<'d>),
///         Spi1(crate::peripherals::Spi1<'d>),
///     }
/// }
/// ```
#[macro_export]
macro_rules! any_peripheral {
    (
        $(#[$outer:meta])*
        $vis:vis peripheral $name:ident <'d> {
            $(
                $(#[$inner:meta])*
                $variant:ident($ty:ty),
            )+
        }
    ) => {
        $(#[$outer])*
        $vis enum $name<'d> {
            $(
                $(#[$inner])*
                $variant($ty),
            )+
        }

        /// Trait for types that can be degraded into an `$name`.
        $vis trait Degrade<'d> {
            fn degrade(self) -> $name<'d>;
        }

        $(
            impl<'d> Degrade<'d> for $ty {
                fn degrade(self) -> $name<'d> {
                    $name::$variant(self)
                }
            }
        )+
    };
}

/// Infallible conversion — used when a conversion cannot fail.
#[macro_export]
macro_rules! infallible {
    ($from:ty => $to:ty, |$val:ident| $expr:expr) => {
        impl From<$from> for $to {
            fn from($val: $from) -> Self {
                $expr
            }
        }
    };
}
