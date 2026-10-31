use super::*;

#[test]
#[should_panic = "cannot retrieve current module context, no module currently in scope"]
fn current_panic_outside_module_ctx() {
    let _ = current();
}