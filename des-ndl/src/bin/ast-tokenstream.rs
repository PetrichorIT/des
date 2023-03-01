use des_ndl::{ast::TokenStream, *};

const TEXT: &str = "
include str;
include ast as ast;

// Comments

module A {
    gates {
        in @input
        out[5] @output
    }
    connections {
        in --> out[1]
    }
}
";

fn main() {
    let mut smap = SourceMap::new();
    let asset = smap.load_raw("raw:srctext", TEXT);

    let ts = TokenStream::new(asset).unwrap();
    println!("{:#?}", ts)
}
