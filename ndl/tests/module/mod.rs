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
        false
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
        false
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
        false
    );

    check_err!(
        *errs[1] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true
    );

    check_err!(
        *errs[2] =>
        ParModuleInvalidSectionIdentifer,
        "Invalid subsection identifier 'colons'. Possibilities are gates / submodules / connections / parameters.",
        true
    );

    check_err!(
        *errs[3] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true
    );

    check_err!(
        *errs[4] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token '123'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true
    );

    check_err!(
        *errs[5] =>
        ParModuleMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are gates / submodules / connections / parameters.",
        true
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
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn dsg1_name_collision() {
    let path = "tests/module/D1_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgModuleNamespaceCollision,
        "Namespace collsion. Allready defined a module with name 'X'.",
        false,
        "Try renaming this module"
    );
}

#[test]
fn tychk_cyclic() {
    let path = "tests/module/T_Cyclic.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'X' via path 'self'.",
        false
    );

    check_err!(
        *errs[1] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'A' via path 'b/c/a'.",
        false
    );

    check_err!(
        *errs[2] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'B' via path 'c/a/b'.",
        false
    );

    check_err!(
        *errs[3] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'C' via path 'a/b/c'.",
        false
    );
}

#[test]
fn tychk_proto_cyclic() {
    let path = "tests/module/T_ProtoCyclic.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'B' via path 'a/b'.",
        false
    );

    check_err!(
        *errs[1] =>
        TycModuleSubmoduleRecrusiveTyDefinition,
        "Cannot create cyclic definition for type 'A' via path 'b/a'.",
        false
    );

    check_err!(
        *errs[2] =>
        DsgProtoImlMissing,
        "Missing prototype impl block for type 'B'.",
        false
    );
}

mod children;
mod connections;
mod gates;
