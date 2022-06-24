use ndl::{Error, ErrorCode::*, NdlResolver};

use crate::check_err;

#[test]
fn dsg_alias_proto_def() {
    let path = "tests/lookalike/D_AliasProtoDef.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        DsgInvalidPrototypeAtAlias,
        "No prototype called 'someP' found for alias 'B'.",
        false,
        "Do you mean 'SomeP'?"
    );

    check_err!(
        *errs[1] =>
        DsgInvalidPrototypeAtAlias,
        "No prototype called 'someb' found for alias 'C'.",
        false,
        "Do you mean 'SomeP'?"
    );

    check_err!(
        *errs[2] =>
        DsgInvalidPrototypeAtAlias,
        "No prototype called 'sonbra' found for alias 'D'. Module 'sonbra' is no prototype.",
        false
    );
}

#[test]
fn dsg_child_node_spec() {
    let path = "tests/lookalike/D_ChildNodeSpec.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 7);

    check_err!(
        *errs[0] =>
        DsgSubmoduleMissingTy,
        "No module with type 'Rauter' found in scope.",
        false,
        "Do you mean 'Router'?"
    );

    check_err!(
        *errs[1] =>
        DsgSubmoduleMissingTy,
        "No module with type 'Routers' found in scope.",
        false,
        "Do you mean 'Router'?"
    );

    check_err!(
        *errs[2] =>
        DsgSubmoduleMissingTy,
        "No module with type 'agger' found in scope.",
        false,
        "Do you mean 'Logger'?"
    );

    check_err!(
        *errs[3] =>
        DsgInvalidPrototypeAtSome,
        "No prototype called 'Aplication' found.",
        false,
        "Do you mean 'Application'?"
    );

    // SUBSYS

    check_err!(
        *errs[4] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'Rauter' found in scope.",
        false,
        "Do you mean 'Router'?"
    );
    check_err!(
        *errs[5] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'Routers' found in scope.",
        false,
        "Do you mean 'Router'?"
    );
    check_err!(
        *errs[6] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'agger' found in scope.",
        false,
        "Do you mean 'Logger'?"
    );
}

#[test]
fn dsg_con_def_module() {
    let path = "tests/lookalike/D_ConDefModule.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    check_err!(
        *errs[0] =>
        DsgConInvalidChannel,
        "Could not find link 'MyLank' in scope.",
        false,
        "Do you mean 'MyLink'?"
    );

    check_err!(
        *errs[1] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'locallyIn' exists on this module.",
        false,
        "Do you mean 'localIn'?".to_string()
    );

    check_err!(
        *errs[2] =>
        DsgConInvalidField,
        "Field 'beckup' was not defined on module 'Test'.",
        false,
        "Do you mean 'backup'?"
    );

    check_err!(
        *errs[3] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'inn' exists on module 'A'.",
        false,
        "Do you mean 'in'?"
    );
}

#[test]
fn dsg_con_def_subsys() {
    let path = "tests/lookalike/D_ConDefSubsys.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    check_err!(
        *errs[0] =>
        DsgConInvalidChannel,
        "Could not find link 'MyLank' in scope.",
        false,
        "Do you mean 'MyLink'?"
    );

    check_err!(
        *errs[1] =>
        DsgConInvalidField,
        "Field 'beckup' was not defined on subsystem 'Testnet'.",
        false,
        "Do you mean 'backup'?"
    );

    check_err!(
        *errs[2] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'inn' exists on module 'A'.",
        false,
        "Do you mean 'in'?"
    );
}
