#![allow(unused_macros)]

macro_rules! cfg_net {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "net")]
            #[cfg_attr(docsrs, doc(cfg(feature = "net")))]
            $item
        )*
    }
}

macro_rules! cfg_async {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "async")]
            #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
            $item
        )*
    }
}

macro_rules! cfg_not_async {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "async"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "async"))))]
            $item
        )*
    }
}

macro_rules! cfg_net_default {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "std-net"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "std-net"))))]
            $item
        )*
    }
}

macro_rules! cfg_net_std {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "std-net")]
            #[cfg_attr(docsrs, doc(cfg(feature = "std-net")))]
            $item
        )*
    }
}

macro_rules! cfg_metrics {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "metrics")]
            #[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
            $item
        )*
    }
}

macro_rules! cfg_cqueue {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "cqueue")]
            #[cfg_attr(docsrs, doc(cfg(feature = "cqueue")))]
            $item
        )*
    }
}

macro_rules! cfg_not_cqueue {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "cqueue"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "cqueue"))))]
            $item
        )*
    }
}
