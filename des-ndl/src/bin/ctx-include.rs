use std::collections::LinkedList;

use des_ndl::*;

fn main() -> Result<()> {
    let ctx = Context::load("des-ndl/src/bin/case-0/main.ndl")?;
    let mut errors = LinkedList::new();
    let tbl = LinkSymbolTable::from_ctx(&ctx, ctx.root.clone(), &mut errors);
    let tbl2 = ModuleSymbolTable::from_ctx(&ctx, ctx.root.clone(), &mut errors);
    println!("{:#?}", tbl);
    println!("{:#?}", tbl2);
    println!("{:#?}", errors);
    Ok(())
}
