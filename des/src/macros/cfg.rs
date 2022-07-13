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

macro_rules! cfg_net_v4 {
    ($($item:item)*) => {
        $(
            #[cfg(not(feature = "net-ipv6"))]
            #[cfg_attr(docsrs, doc(cfg(not(feature = "net-ipv6"))))]
            $item
        )*
    }
}

macro_rules! cfg_net_v6 {
    ($($item:item)*) => {
        $(
            #[cfg(feature = "net-ipv6")]
            #[cfg_attr(docsrs, doc(cfg(feature = "net-ipv6")))]
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
