use des_ndl::*;

const TEXT: &str = "
include str/a/b/c
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    let buf = ParseBuffer::new(asset, ts);
    let include = Include::parse(&buf);

    println!("{include:#?}")
}
