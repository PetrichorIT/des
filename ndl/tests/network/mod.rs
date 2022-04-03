use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_no_ident() {
    let path = "tests/network/P_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        ParNetworkMissingIdentifer,
        "Invalid token '{'. Expected network identifier.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
}

#[test]
fn par_missing_block_open() {
    let path = "tests/network/P_MissingBlockOpen.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 5);

    check_err!(
        *errs[0] =>
        ParNetworkMissingDefBlockOpen,
        "Invalid token 'nodes'. Expected network definition block (OpenBrace).",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
    assert!(errs[4].transient);
}

#[test]
fn par_unexpected_subsection() {
    let path = "tests/network/P_UnexpectedSubsection.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 6);

    check_err!(
        *errs[0] =>
        ParNetworkInvalidSectionIdentifer,
        "Invalid subsection identifier 'colons'. Possibilities are nodes / connections / parameters.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParNetworkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[2] =>
        ParNetworkMissingSectionIdentifier,
        "Invalid token '123'. Expected identifier for subsection are nodes / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[3] =>
        ParNetworkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[4] =>
        ParNetworkInvalidSectionIdentifer,
        "Invalid subsection identifier 'submodules'. Possibilities are nodes / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[5] =>
        ParNetworkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters.",
        true,
        None
    );
}

#[test]
fn par_subsection_no_colon() {
    let path = "tests/network/P_SubsectionNoColon.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParNetworkInvalidSeperator,
        "Unexpected token 'a'. Expected colon ':'.",
        false,
        None
    );

    assert_eq!(r.gtyctx_def().network("A").unwrap().nodes.len(), 2);
}

#[test]
fn dsg1_name_collision() {
    let path = "tests/network/D1_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgDefNameCollision,
        "Cannot create two networks with name 'X'.",
        false,
        Some(ErrorSolution::new("Try renaming this network".to_string(), Loc::new(60, 29, 8)))
    );
}
