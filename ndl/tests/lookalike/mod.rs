use ndl::{Error, ErrorCode::*, ErrorSolution, Loc, NdlResolver};

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
        Some(ErrorSolution::new("Do you mean 'SomeP'?".to_string(), Loc::new(10,62,1)))
    );

    check_err!(
        *errs[1] =>
        DsgInvalidPrototypeAtAlias,
        "No prototype called 'someb' found for alias 'C'.",
        false,
        Some(ErrorSolution::new("Do you mean 'SomeP'?".to_string(), Loc::new(10,62,1)))
    );

    check_err!(
        *errs[2] =>
        DsgInvalidPrototypeAtAlias,
        "No prototype called 'sonbra' found for alias 'D'. Module 'sonbra' is no prototype.",
        false,
        None
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
        Some(ErrorSolution::new("Do you mean 'Router'?".to_string(), Loc::new(7,9,1)))
    );

    check_err!(
        *errs[1] =>
        DsgSubmoduleMissingTy,
        "No module with type 'Routers' found in scope.",
        false,
        Some(ErrorSolution::new("Do you mean 'Router'?".to_string(), Loc::new(7,9,1)))
    );

    check_err!(
        *errs[2] =>
        DsgSubmoduleMissingTy,
        "No module with type 'agger' found in scope.",
        false,
        Some(ErrorSolution::new("Do you mean 'Logger'?".to_string(), Loc::new(25,9,2)))
    );

    check_err!(
        *errs[3] =>
        DsgInvalidPrototypeAtSome,
        "No prototype called 'Aplication' found.",
        false,
        Some(ErrorSolution::new("Do you mean 'Application'?".to_string(), Loc::new(46,14,3)))
    );

    // SUBSYS

    check_err!(
        *errs[4] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'Rauter' found in scope.",
        false,
        Some(ErrorSolution::new("Do you mean 'Router'?".to_string(), Loc::new(7,9,1)))
    );
    check_err!(
        *errs[5] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'Routers' found in scope.",
        false,
        Some(ErrorSolution::new("Do you mean 'Router'?".to_string(), Loc::new(7,9,1)))
    );
    check_err!(
        *errs[6] =>
        DsgSubmoduleMissingTy,
        "No module or subsystem with name 'agger' found in scope.",
        false,
        Some(ErrorSolution::new("Do you mean 'Logger'?".to_string(), Loc::new(25,9,2)))
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
        Some(ErrorSolution::new("Do you mean 'MyLink'?".to_string(), Loc::new(5,66,1)))
    );

    check_err!(
        *errs[1] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'locallyIn' exists on this module.",
        false,
        Some(ErrorSolution::new("Do you mean 'localIn'?".to_string(), Loc::new(179,8,15)))
    );

    check_err!(
        *errs[2] =>
        DsgConInvalidField,
        "Field 'beckup' was not defined on module 'Test'.",
        false,
        Some(ErrorSolution::new("Do you mean 'backup'?".to_string(), Loc::new(310, 9, 22)))
    );

    check_err!(
        *errs[3] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'inn' exists on module 'A'.",
        false,
        Some(ErrorSolution::new("Do you mean 'in'?".to_string(), Loc::new(107, 3, 9)))
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
        Some(ErrorSolution::new("Do you mean 'MyLink'?".to_string(), Loc::new(5,66,1)))
    );

    check_err!(
        *errs[1] =>
        DsgConInvalidField,
        "Field 'beckup' was not defined on subsystem 'Testnet'.",
        false,
        Some(ErrorSolution::new("Do you mean 'backup'?".to_string(), Loc::new(154, 421, 13)))
    );

    check_err!(
        *errs[2] =>
        DsgConInvalidLocalGateIdent,
        "No local gate cluster 'inn' exists on module 'A'.",
        false,
        Some(ErrorSolution::new("Do you mean 'in'?".to_string(), Loc::new(107, 3, 9)))
    );
}
