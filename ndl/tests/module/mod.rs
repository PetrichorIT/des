use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_no_ident() {
    let path = "tests/module/P_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 6);

    check_err!(
        *errs[0] =>
        ParModuleMissingIdentifer,
        "Invalid token '{'. Expected module identfier.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
    assert!(errs[4].transient);
    assert!(errs[5].transient);
}

#[test]
fn par_missing_block_open() {
    let path = "tests/module/P_MissingBlockOpen.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 5);

    check_err!(
        *errs[0] =>
        ParModuleMissingDefBlockOpen,
        "Invalid token 'gates'. Expected module definition block (OpenBrace).",
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
    let path = "tests/module/P_UnexpectedSubsection.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 6);

    check_err!(
        *errs[0] =>
        ParModuleInvalidSectionIdentifer,
        "Invalid subsection identifier 'nodes'. Possibilities are gates / submodules / connections / parameters.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[2] =>
        ParModuleInvalidSectionIdentifer,
        "Invalid subsection identifier 'colons'. Possibilities are gates / submodules / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[3] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[4] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token '123'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true,
        None
    );

    check_err!(
        *errs[5] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true,
        None
    );
}

#[test]
fn par_subsection_no_colon() {
    let path = "tests/module/P_SubsectionNoColon.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleInvalidSeperator,
        "Unexpected token 'in'. Expected colon ':'.",
        false,
        None
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

mod children;
mod connections;
mod gates;
