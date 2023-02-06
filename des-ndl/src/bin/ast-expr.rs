use des_ndl::*;

const TEXT: &str = "
connections {
    gate <-- fastlink <-- gute
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let expr = ConnectionsStmt::parse(&buf).unwrap();

    println!("{expr:#?}");
    // for entry in expr.iter() {
    //     println!("- {entry:?}")
    // }
}
