use crate::check_err;
use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn par_missing_ident() {
    let path = "tests/link/P_MissingIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        ParLinkMissingIdentifier,
        "Unexpected token '{'. Expected identifer for link definition.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
    assert!(errs[3].transient);
}

#[test]
fn par_missing_block_open() {
    let path = "tests/link/P_MissingBlockOpen.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        ParLinkMissingDefBlockOpen,
        "Unexpected token 'latency'. Expected block for link definition.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);
}

#[test]
fn par_invalid_key() {
    let path = "tests/link/P_InvalidKey.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 8);

    check_err!(
        *errs[0] =>
        ParLinkInvalidKeyToken,
        "Unexpected token '123'. Expected identifer for definition key.",
        false,
        None
    );

    assert!(errs[1].transient);
    assert!(errs[2].transient);

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 1_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.9);

    check_err!(
        *errs[3] =>
        ParLinkIncompleteDefinition,
        "Channel 'A' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters bitrate".to_string(), Loc::new(5, 1, 1)))
    );

    // Case 2

    check_err!(
        *errs[4] =>
        ParLinkInvalidKey,
        "Invalid key 'byterate' in kv-pair. Valid keys are latency, bitrate or jitter.",
        false,
        None
    );

    assert!(errs[5].transient);
    assert!(errs[6].transient);

    assert_eq!(r.gtyctx_def().link("B").unwrap().bitrate, 1_000);
    assert_eq!(r.gtyctx_def().link("B").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("B").unwrap().latency, 0.9);

    check_err!(
        *errs[7] =>
        ParLinkIncompleteDefinition,
        "Channel 'B' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters bitrate".to_string(), Loc::new(65, 1, 7)))
    );
}

#[test]
fn par_missing_seperator() {
    let path = "tests/link/P_MissingSeperator.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParLinkInvalidKvSeperator,
        "Unexpected token '0.9'. Expected colon ':' between definition key and value.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParLinkIncompleteDefinition,
        "Channel 'A' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters latency".to_string(), Loc::new(5, 1, 1)))
    );

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 10_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.1);
}

#[test]
fn par_no_literal() {
    let path = "tests/link/P_NoLiteral.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParLinkInvalidValueToken,
        "Unexpected token 'ident'. Expected literal.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParLinkIncompleteDefinition,
        "Channel 'A' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters bitrate".to_string(), Loc::new(5, 1, 1)))
    );

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 1_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.9);
}

#[test]
fn par_invalid_literal_ty() {
    let path = "tests/link/P_InvalidLitTy.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        ParLinkInvalidValueType,
        "Invalid value type. Expected integer.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParLinkIncompleteDefinition,
        "Channel 'A' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters bitrate".to_string(), Loc::new(5, 1, 1)))
    );

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 1_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.9);

    // Case 2

    check_err!(
        *errs[2] =>
        ParLinkInvalidValueType,
        "Invalid value type. Expected float.",
        false,
        None
    );

    check_err!(
        *errs[3] =>
        ParLinkIncompleteDefinition,
        "Channel 'B' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters latency".to_string(), Loc::new(68, 1, 7)))
    );

    assert_eq!(r.gtyctx_def().link("B").unwrap().bitrate, 10_000);
    assert_eq!(r.gtyctx_def().link("B").unwrap().jitter, 0.6);
    assert_eq!(r.gtyctx_def().link("B").unwrap().latency, 0.1);
}

#[test]
fn par_literal_parse_error() {
    let path = "tests/link/P_LiteralParseError.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        ParLiteralFloatParseError,
        "Float parsing error: invalid float literal.",
        false,
        None
    );

    check_err!(
        *errs[1] =>
        ParLiteralIntParseError,
        "Int parsing error: invalid digit found in string.",
        false,
        None
    );

    check_err!(
        *errs[2] =>
        ParLinkIncompleteDefinition,
        "Channel 'A' was missing some parameters.",
        true,
        Some(ErrorSolution::new("Add parameters bitrate + jitter".to_string(), Loc::new(5, 1, 1)))
    );

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 1_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.1);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.9);
}

#[test]
fn par_cost() {
    let path = "tests/link/P_Cost.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    //dbg!(&r);

    assert_eq!(r.gtyctx_def().link("A").unwrap().bitrate, 10_000);
    assert_eq!(r.gtyctx_def().link("A").unwrap().jitter, 0.1);
    assert_eq!(r.gtyctx_def().link("A").unwrap().latency, 0.1);

    assert_eq!(r.gtyctx_def().link("A").unwrap().cost, 2.0);
}

#[test]
fn dsg1_name_collision() {
    let path = "tests/link/D1_NameCollision.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    check_err!(
        *errs[0] =>
        DsgDefNameCollision,
        "Cannot create two links with name 'A'.",
        false,
        Some(ErrorSolution::new("Try renaming this link".to_string(), Loc::new(71, 60, 7)))
    );
}
