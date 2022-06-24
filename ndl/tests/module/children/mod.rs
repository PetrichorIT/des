use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_cluster_invalid_dots() {
    let path = "tests/module/children/P_ClusterDots.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 5);

    check_err!(
        *errs[0] =>
        ParModuleSubInvalidClusterDotChain,
        "Unexpected token '3'. Expected three dots.",
        false
    );

    check_err!(
        *errs[1] =>
        ParModuleSubInvalidClusterDotChain,
        "Unexpected token ']'. Expected three dots.",
        false
    );

    assert!(errs[2].transient);
    assert!(errs[3].transient);
    assert!(errs[4].transient);
}

#[test]
fn par_cluster_no_closing() {
    let path = "tests/module/children/P_ClusterNoClosing.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleSubMissingClosingBracket,
        "Unexpected token ':'. Expected closing bracket.",
        false,
        "Try adding ']'"
    );
}

#[test]
fn par_no_colon() {
    let path = "tests/module/children/P_MissingColon.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParModuleSubInvalidSeperator,
        "Unexpected token 'X'. Expected colon ':'.",
        false,
        "Try adding ':'"
    );

    check_err!(
        *errs[1] =>
        ParModuleSubInvalidSeperator,
        "Unexpected token 'X'. Expected colon ':'.",
        false,
        "Try adding ':'"
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().submodules.len(), 2)
}

#[test]
fn par_no_ty() {
    let path = "tests/module/children/P_NoTy.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleSubInvalidIdentiferToken,
        "Unexpected token ','. Expected type identifer.",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().submodules.len(), 1)
}

#[test]
fn dsg1_name_collision() {
    let path = "tests/module/children/D1_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgSubmoduleNamespaceCollision,
        "Namespace collision. Allready defined a submodule with name 'a' on module 'Y'.",
        false
    );
}
