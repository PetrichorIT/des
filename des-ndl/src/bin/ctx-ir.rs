use des_ndl::*;

fn main() {
    let ctx = match Context::load("des-ndl/src/bin/case-1/main.ndl") {
        Ok(ctx) => ctx,
        Err(e) => {
            println!("[{e}]");
            return;
        }
    };
    // println!("{:#?}", ctx.ir);
    println!("{:#?}", ctx.entry);
}
