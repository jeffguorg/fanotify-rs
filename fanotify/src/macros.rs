/*
    Copied from nix crate and modified to allow multiple patterns in a single block.

    This simplifies flag groups definition.
*/
macro_rules! fa_bitflags {
    // modified: accept a list of pub struct, force cast to T
    (
        // first
        $(#[$outer:meta])*
        pub struct $BitFlags:ident: ~$T:ty {
            $(
                $(#[$inner:ident $($args:tt)*])*
                $Flag:ident;
            )+
        }

        // modified part: match rest
        $($t:tt)*
    ) => {
        ::bitflags::bitflags! {
            #[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            #[repr(transparent)]
            $(#[$outer])*
            pub struct $BitFlags: $T {
                $(
                    $(#[$inner $($args)*])*
                    // always cast to $T
                    const $Flag = libc::$Flag as $T;
                )+
            }
        }

        // modified part: recursively handle rest structs
        fa_bitflags! {
            $($t)*
        }
    };

    // from nix: input: accept a list of pub struct 
    (
        // first
        $(#[$outer:meta])*
        pub struct $BitFlags:ident: $T:ty {
            $(
                $(#[$inner:ident $($args:tt)*])*
                $Flag:ident $(as $cast:ty)*;
            )+
        }

        // modified part: match rest
        $($t:tt)*
    ) => {
        // generate bitflags struct
        ::bitflags::bitflags! {
            #[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
            #[repr(transparent)]
            $(#[$outer])*
            pub struct $BitFlags: $T {
                $(
                    $(#[$inner $($args)*])*
                    const $Flag = libc::$Flag $(as $cast)*;
                )+
            }
        }

        // modified part: recursively handle rest structs
        fa_bitflags! {
            $($t)*
        }
    };

    // modified part: empty block don't produce anything.
    () => {}
}