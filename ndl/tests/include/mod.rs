use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn base() {
    let path = "tests/include";
    let mut r = NdlResolver::new(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 2);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgIncludeInvalidAlias,
        "Include 'Third' cannot be resolved. No such file exists.",
        false,
        None
    );
}
