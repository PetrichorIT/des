use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_no_ident() {
    let path = "tests/module/gates/P_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleGateInvalidIdentifierToken,
        "Invalid token '123'. Expected gate identifier.",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_unexpected_token() {
    let path = "tests/module/gates/P_UnexpectedToken.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleGateInvalidIdentifierToken,
        "Unexpected token '/'. Expected whitespace.",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_cluster_no_closing() {
    let path = "tests/module/gates/P_ClusterNoClosing.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleGateMissingClosingBracket,
        "Unexpected token 'out'. Expected closing bracket.",
        false,
        "Try adding ']'"
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_literal_parse_error() {
    let path = "tests/module/gates/P_LiteralParseError.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParLiteralIntParseError,
        "Failed to parse integer: invalid digit found in string.",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_literal_wrong_ty() {
    let path = "tests/module/gates/P_LiteralWrongTy.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleGateInvalidGateSize,
        "Unexpected token '1.0'. Expected gate size (Int).",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_cluster_no_literal() {
    let path = "tests/module/gates/P_ClusterNoLiteral.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        ParModuleGateInvalidGateSize,
        "Unexpected token 'ident'. Expected gate size (Int).",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 2);
}

#[test]
fn par_invalid_annotation() {
    let path = "tests/module/gates/P_InvalidAnnotation.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParModuleGateInvalidServiceAnnotation,
        "Invalid service annotation 'inputty'.",
        false,
        "Remove or replace with 'input' or 'output'"
    );

    check_err!(
        *errs[1] =>
        ParModuleGateInvalidServiceAnnotation,
        "Invalid token '123', expected ident.",
        false
    );

    assert_eq!(r.gtyctx_def().module("A").unwrap().gates.len(), 3);
}

#[test]
fn dsg1_invalid_gate_size() {
    let path = "tests/module/gates/D1_InvalidGateSize.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgGateNullSize,
        "Cannot create gate 'input' with size 0.",
        false
    );
}

#[test]
fn tychk_name_collision() {
    let path = "tests/module/gates/T_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgGateNamespaceCollision,
        "Namespace collision. Allready defined a gate with name 'in' on module 'A'.",
        false,
        "Try renaming this gate"
    );
}
