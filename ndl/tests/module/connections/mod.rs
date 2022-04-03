use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_con_ident_no_ident() {
    let path = "tests/module/connections/P_ConIdent_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        ParModuleConInvalidIdentiferToken,
        "Unexpected token '-'. Expected identifer.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 1);
}

#[test]
fn par_cluster_ident_no_closing() {
    let path = "tests/module/connections/P_ClusterIdent_NoClosing.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleConMissingClosingBracketForCLusterIdent,
        "Missing closing bracket for clustered ident.",
        false,
        Some(ErrorSolution::new("Try adding ']'".to_string(), Loc::new(163, 0, 14)))
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().connections.len(), 1)
}

#[test]
fn par_sub_no_ident() {
    let path = "tests/module/connections/P_Sub_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleConInvalidIdentiferToken,
        "Unexpected token. Expected second part identifer.",
        false,
        None
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().connections.len(), 0)
}

#[test]
fn par_clustered_gate_no_closing() {
    let path = "tests/module/connections/P_ClusterGate_NoClosing.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParModuleConMissingClosingBracketForCLusterIdent,
        "Missing closing bracket for clustered gate ident.",
        false,
        Some(ErrorSolution::new("Try adding ']'".to_string(), Loc::new(249, 0, 17)))
    );
    check_err!(
        *errs[1] =>
        ParModuleConMissingClosingBracketForCLusterIdent,
        "Missing closing bracket for clustered gate ident.",
        false,
        Some(ErrorSolution::new("Try adding ']'".to_string(), Loc::new(281, 0, 18)))
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().connections.len(), 3)
}

#[test]
fn par_no_slash_or_whitespace() {
    let path = "tests/module/connections/P_NoSlashOrWhitespace.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 7);

    check_err!(
        *errs[0] =>
        ParModuleConInvalidIdentiferToken,
        "Unexpected token '-'. Expected whitespace or slash.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);

    check_err!(
        *errs[4] =>
        ParModuleConInvalidIdentiferToken,
        "Unexpected token '-'. Expected whitespace or slash.",
        false,
        None
    );

    assert!(errs[5].transient);
    assert!(errs[6].transient);

    assert_eq!(r.gtyctx_def().module("A").unwrap().connections.len(), 1)
}

#[test]
fn par_arrow_direction_missmatch() {
    let path = "tests/module/connections/P_ArrowDirectionMissmatch.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleConInvaldiChannelSyntax,
        "Invalid arrow syntax. Both arrows must match.",
        false,
        None
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().connections.len(), 0)
}
