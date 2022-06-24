use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_exports() {
    let path = "tests/subsystem/P_Exports.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(!r.ectx.has_errors());
}

#[test]
fn par_exports_no_sep() {
    let path = "tests/subsystem/P_ExportsNoSep.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());
    let errs = r.ectx.all().collect::<Vec<&Error>>();

    check_err!(
        *errs[0] =>
        ParSubsystemExportsInvalidSeperatorToken,
        "Unexpected token 'out'. Expected seperator '/'.",
        false
    );
}

#[test]
fn par_no_ident() {
    let path = "tests/subsystem/P_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        ParSubsystemMissingIdentifer,
        "Invalid token '{'. Expected network identifier.",
        false
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
}

#[test]
fn par_missing_block_open() {
    let path = "tests/subsystem/P_MissingBlockOpen.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 5);

    check_err!(
        *errs[0] =>
        ParSubsystemMissingDefBlockOpen,
        "Invalid token 'nodes'. Expected network definition block (OpenBrace).",
        false
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
    assert!(errs[4].transient);
}

#[test]
fn par_unexpected_subsection() {
    let path = "tests/subsystem/P_UnexpectedSubsection.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 6);

    check_err!(
        *errs[0] =>
        ParSubsystemInvalidSectionIdentifer,
        "Invalid subsection identifier 'colons'. Possibilities are nodes / connections / parameters / exports.",
        false
    );

    check_err!(
        *errs[1] =>
        ParSubsystemkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters / exports.",
        true
    );

    check_err!(
        *errs[2] =>
        ParSubsystemkMissingSectionIdentifier,
        "Invalid token '123'. Expected identifier for subsection are nodes / connections / parameters / exports.",
        true
    );

    check_err!(
        *errs[3] =>
        ParSubsystemkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters / exports.",
        true
    );

    check_err!(
        *errs[4] =>
        ParSubsystemInvalidSectionIdentifer,
        "Invalid subsection identifier 'submodules'. Possibilities are nodes / connections / parameters / exports.",
        true
    );

    check_err!(
        *errs[5] =>
        ParSubsystemkMissingSectionIdentifier,
        "Invalid token ':'. Expected identifier for subsection are nodes / connections / parameters / exports.",
        true
    );
}

#[test]
fn par_subsection_no_colon() {
    let path = "tests/subsystem/P_SubsectionNoColon.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParSubsystemInvalidSeperator,
        "Unexpected token 'a'. Expected colon ':'.",
        false
    );

    assert_eq!(r.gtyctx_def().subsystem("A").unwrap().nodes.len(), 2);
}

#[test]
fn dsg1_name_collision() {
    let path = "tests/subsystem/D1_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgModuleNamespaceCollision,
        "Namespace collsion. Allready defined a subsystem with name 'X'.",
        false,
        "Try renaming this network"
    );
}

#[test]
fn dsg2_netinnet() {
    let path = "tests/subsystem/D2_NetInNet.ndl";
    let mut r = NdlResolver::new_with(
        path,
        NdlResolverOptions {
            silent: true,
            verbose: true,
            verbose_output_dir: "tests/subsystem/d2_netinnet/".into(),
            desugar: true,
            tychk: true,
        },
    )
    .expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(!r.ectx.has_errors());
}

#[test]
fn dsg_nested_subysys() {
    let path = "tests/subsystem/D_NestedSubsys.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        DsgConInvalidField,
        "Field 'error' was not defined on subsystem 'FieldError'.",
        false
    );

    check_err!(
        *errs[1] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'error' exists on subsystem 'Sub'.",
        false
    );
}

#[test]
fn tychk_invalid_sub_ty() {
    let path = "tests/subsystem/T_InvalidSubmodule.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'B' found in scope.",
        false
    );
}

#[test]
fn tychk_empty() {
    let path = "tests/subsystem/T_Empty.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        TycNetworkEmptyNetwork,
        "Network 'A' does not contain any nodes.",
        false
    );
}
