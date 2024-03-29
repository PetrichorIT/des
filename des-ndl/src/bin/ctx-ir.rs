use des_ndl::*;

fn main() {
    let mut ctx = match Context::load("des-ndl2/blueprint.ndl") {
        Ok(ctx) => ctx,
        Err(e) => {
            println!("[{e}]");
            return;
        }
    };
    let entry = ctx.entry.take();
    drop(ctx);

    println!("{:#?}", entry);
    drop(entry);
}
