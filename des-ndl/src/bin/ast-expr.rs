use des_ndl::*;

const TEXT: &str = "
gates {
    in @input,
    out,
    clusi[5] @input,
    clusa[1],
    debug
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = GatesStmt::parse(&buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
