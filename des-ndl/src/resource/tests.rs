use super::*;

const FILE_A: &'static str = "0123456789"; // 10 bytes, 1 line
const FILE_B: &'static str = "A\nBB\nCCC\nDDDD\nEEE\nFF\nG"; // 22 bytes, 7 lines
const FILE_C: &'static str = "module A {\nB\n}\n"; // 15 bytes, 4 lines

#[test]
fn source_map_loading() {
    let mut map = SourceMap::new();

    let a = map.load_raw(AssetIdentifier::raw("a"), FILE_A);
    assert_eq!(a.alias(), "a");
    assert_eq!(a.source(), FILE_A);
    assert_eq!(a.source_span(), Span::new(0, 10));

    assert_eq!(
        map.asset_for(Span::new(0, 7)).map(|v| v.ident.alias()),
        Some("a")
    );
    assert_eq!(
        map.asset_for(Span::new(3, 2)).map(|v| v.ident.alias()),
        Some("a")
    );
    assert_eq!(
        map.asset_for(Span::new(5, 5)).map(|v| v.ident.alias()),
        Some("a")
    );
    assert_eq!(
        map.asset_for(Span::new(5, 7)).map(|v| v.ident.alias()),
        None
    );

    let b = map.load_raw(AssetIdentifier::raw("b"), FILE_B);
    assert_eq!(b.alias(), "b");
    assert_eq!(b.source(), FILE_B);
    assert_eq!(b.source_span(), Span::new(10, 22));

    assert_eq!(
        map.asset_for(Span::new(10, 7)).map(|v| v.ident.alias()),
        Some("b")
    );
    assert_eq!(
        map.asset_for(Span::new(15, 10)).map(|v| v.ident.alias()),
        Some("b")
    );
    assert_eq!(
        map.asset_for(Span::new(20, 12)).map(|v| v.ident.alias()),
        Some("b")
    );
    assert_eq!(
        map.asset_for(Span::new(5, 20)).map(|v| v.ident.alias()),
        None
    );
    assert_eq!(
        map.asset_for(Span::new(31, 5)).map(|v| v.ident.alias()),
        None
    );

    let c = map.load_raw(AssetIdentifier::raw("c"), FILE_C);
    assert_eq!(c.alias(), "c");
    assert_eq!(c.source(), FILE_C);
    assert_eq!(c.source_span(), Span::new(32, 15));

    assert_eq!(
        map.asset_for(Span::new(32, 7)).map(|v| v.ident.alias()),
        Some("c")
    );
    assert_eq!(
        map.asset_for(Span::new(37, 8)).map(|v| v.ident.alias()),
        Some("c")
    );
    assert_eq!(
        map.asset_for(Span::new(32, 12)).map(|v| v.ident.alias()),
        Some("c")
    );
    assert_eq!(
        map.asset_for(Span::new(32, 20)).map(|v| v.ident.alias()),
        None
    );
    assert_eq!(
        map.asset_for(Span::new(31, 5)).map(|v| v.ident.alias()),
        None
    );
}

const EX0: &'static str = "include str;
include x;

module M {
    gates {
        in[5] @input
    }

    connections {
        in -->
    }
}

entry M;
// Comment befor EOF
";

#[test]
fn source_map_padded_span() {
    let mut smap = SourceMap::new();
    smap.load_raw("raw:ex0", EX0);

    // Full padding.
    let span = EX0.bytes().enumerate().find(|c| c.1 == b'5').unwrap().0;
    let span = Span::new(span, 1);

    let (padded_slice, offset) = smap.slice_padded_for(span);
    assert_eq!(offset, 2);
    assert_eq!(
        padded_slice,
        "module M {\n    gates {\n        in[5] @input\n    }\n"
    );

    // Trunc end
    let span = EX0.bytes().enumerate().find(|c| c.1 == b'y').unwrap().0;
    let span = Span::new(span, 1);

    let (padded_slice, offset) = smap.slice_padded_for(span);
    assert_eq!(offset, 2);
    assert_eq!(padded_slice, "}\n\nentry M;\n// Comment befor EOF");

    // Trunc start (offset 1)
    let span = EX0.bytes().enumerate().find(|c| c.1 == b'x').unwrap().0;
    let span = Span::new(span, 1);

    let (padded_slice, offset) = smap.slice_padded_for(span);
    assert_eq!(offset, 1);
    assert_eq!(padded_slice, "include str;\ninclude x;\n\nmodule M {");
}
