use ndl::ErrorCode::*;
use ndl::*;

use crate::check_err;

#[test]
fn base() {
    let mut resolver =
        NdlResolver::new("tests/prototype/base").expect("Failed to create resolver.");

    println!("{}", resolver);

    let _ = resolver.run();

    println!("{}", resolver);
    assert!(!resolver.ectx.has_errors())
}

///
/// P
///

#[test]
fn par_proto_is_proto() {
    let path = "tests/prototype/P_Proto_IsProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(!r.ectx.has_errors());
    assert!(r.gtyctx_def().module("A").unwrap().is_prototype);
    assert!(r.gtyctx_spec().module("A").is_none());
}

#[test]
fn par_proto_failed_at_network() {
    let path = "tests/prototype/P_Proto_Network.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 5);

    assert_eq!(*errs[0], Error::new(
            ParModuleInvalidSectionIdentifer,
            "Invalid subsection identifier 'nodes'. Possibilities are gates / submodules / connections / parameters.".to_string(),
            Loc::new(45, 5, 4),
            false,
        ));

    assert!(errs[1].transient);
    assert_eq!(errs[1].code, ParModuleMissingSectionIdentifier);
    assert!(errs[2].transient);
    assert_eq!(errs[2].code, ParModuleInvalidSectionIdentifer);
    assert!(errs[3].transient);
    assert_eq!(errs[3].code, ParModuleMissingSectionIdentifier);
    assert!(errs[4].transient);
    assert_eq!(errs[4].code, ParModuleInvalidSectionIdentifer);
}

#[test]
fn par_alias_no_ident() {
    let path = "tests/prototype/P_Alias_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    assert_eq!(
        *errs[0],
        Error::new(
            ParAliasMissingIdent,
            "Unexpected token 'like'. Expected ident.".to_string(),
            Loc::new(22, 4, 3),
            false,
        )
    );

    assert!(errs[1].transient);
    assert_eq!(errs[1].code, ParUnexpectedKeyword);

    assert_eq!(
        *errs[2],
        Error::new(
            ParAliasMissingIdent,
            "Unexpected token '='. Expected ident.".to_string(),
            Loc::new(36, 1, 5),
            false,
        )
    );

    assert!(errs[3].transient);
    assert_eq!(errs[3].code, ParUnexpectedKeyword);
}

#[test]
fn par_alias_like_keyword_invalid() {
    let path = "tests/prototype/P_Alias_LikeToken.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    assert_eq!(
        *errs[0],
        Error::new(
            ParAliasMissingLikeKeyword,
            "Unexpected keyword 'laik'. Expected 'like'.".to_string(),
            Loc::new(24, 4, 3),
            false,
        )
    );

    assert!(errs[1].transient);
    assert_eq!(errs[1].code, ParUnexpectedKeyword);

    assert_eq!(
        *errs[2],
        Error::new(
            ParAliasMissingLikeToken,
            "Unexpected token '1234'. Expected 'like'.".to_string(),
            Loc::new(39, 4, 4),
            false,
        )
    );

    assert!(errs[3].transient);
    assert_eq!(errs[3].code, ParUnexpectedKeyword);
}

#[test]
fn par_alias_no_tyident() {
    let path = "tests/prototype/P_Alias_NoTy.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    assert_eq!(
        *errs[0],
        Error::new(
            ParAliasMissingPrototypeIdent,
            "Unexpected keyword 'module'. Expected prototype ident.".to_string(),
            Loc::new(30, 6, 4),
            false,
        )
    );
    assert_eq!(
        *errs[1],
        Error::new(
            ParAliasMissingPrototypeIdent,
            "Unexpected keyword 'alias'. Expected prototype ident.".to_string(),
            Loc::new(56, 5, 7),
            false,
        )
    );
    assert_eq!(
        *errs[2],
        Error::new(
            ParAliasMissingPrototypeIdent,
            "Unexpected token '123'. Expected prototype ident.".to_string(),
            Loc::new(85, 3, 10),
            false,
        )
    );
}

#[test]
fn par_pimpl_def_and_impl() {
    let path = "tests/prototype/P_Impl_DefAndImpl.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    assert_eq!(
        *errs[0],
        Error::new(
            ParProtoImplAtSomeDef,
            "Unexpected token '{'. Cannot add prototype impl block after use of keyword 'some'."
                .to_string(),
            Loc::new(74, 1, 7),
            false,
        )
    );
}

#[test]
fn par_pimpl_no_ident() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Impl_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            ParProtoImplInvalidIdent,
            "Unexpected token '123'. Expected ident.".to_string(),
            Loc::new(118, 3, 12),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'x'.".to_string(),
            Loc::new(111, 4, 12),
            false,
        )
    );
}

#[test]
fn par_pimpl_no_eq() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Impl_NoEq.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    assert_eq!(
        *errs[0],
        Error::new(
            ParProtoImplExpectedEq,
            "Unexpected token 'is'. Expected '='.".to_string(),
            Loc::new(140, 2, 13),
            false,
        )
    );

    assert!(errs[1].transient);

    assert_eq!(
        *errs[2],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'inner'.".to_string(),
            Loc::new(115, 4, 12),
            false,
        )
    );
}

#[test]
fn par_pimpl_no_ty_ident() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Impl_NoTy.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    // Case 1

    check_err!(
        *errs[0] =>
        ParProtoImplInvalidIdent,
        "Unexpected token '123'. Expected type ident.",
        false,
        None
    );

    check_err!(
        *errs[2] =>
        DsgProtoImplMissingField,
        "Missing prototype impl field 'inner'.",
        false,
        None
    );

    assert_eq!(r.gtyctx_spec().network("Y").unwrap().nodes.len(), 2);
    assert_eq!(
        r.gtyctx_spec().network("Y").unwrap().nodes[0]
            .proto_impl
            .as_ref()
            .unwrap()
            .sorted
            .len(),
        1
    );

    // Case 2
    check_err!(
        *errs[1] =>
        ParProtoImplInvalidIdent,
        "Unexpected token '}'. Expected type ident.",
        false,
        None
    );

    check_err!(
        *errs[3] =>
        DsgProtoImplMissingField,
        "Missing prototype impl field 'inner'.",
        false,
        None
    );

    assert_eq!(r.gtyctx_spec().network("Y2").unwrap().nodes.len(), 2);
    assert_eq!(
        r.gtyctx_spec().network("Y2").unwrap().nodes[0]
            .proto_impl
            .as_ref()
            .unwrap()
            .sorted
            .len(),
        1
    );
}

#[test]
fn par_some_in_network() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Some_Network.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    check_err!(
        *errs[0] =>
        ParNetworkDoesntAllowSome,
        "Unexpected keyword 'some'. This is not allowed on network definitions.",
        false,
        None
    );

    assert!(errs[1].transient);
}

#[test]
fn par_some_no_proto() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Some_NoProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            ParModuleSubInvalidIdentiferToken,
            "Unexpected token '123'. Expected prototype identifer.".to_string(),
            Loc::new(54, 4, 5),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            ParModuleSubInvalidIdentiferToken,
            "Unexpected keyword 'gates'. Expected prototype identifer.".to_string(),
            Loc::new(105, 4, 10),
            false,
        )
    );

    assert_eq!(r.gtyctx_spec().module("X2").unwrap().gates.len(), 2);
}

///
/// D2
///

#[test]
fn dsg2_alias_chk_no_proto() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D2_AliasChk_NoProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            DsgInvalidPrototypeAtAlias,
            "No prototype called 'B' found. Module 'B' is no prototype.".to_string(),
            Loc::new(44, 5, 5),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            DsgInvalidPrototypeAtAlias,
            "No prototype called 'C' found.".to_string(),
            Loc::new(59, 5, 6),
            false,
        )
    );
}

#[test]
fn dsg2_alias_chk_need_include() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D2_AliasChk_Include";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 2);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    assert_eq!(
        *errs[0],
        Error::new_with_solution(
            DsgInvalidPrototypeAtAlias,
            "No prototype called 'A' found.".to_string(),
            Loc::new(0, 5, 1),
            false,
            ErrorSolution::new("Try including 'Other'".to_string(), Loc::new(0, 1, 1))
        )
    );
}

#[test]
fn dsg2_some_chk_no_proto() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D2_SomeChk_NoProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            DsgInvalidPrototypeAtSome,
            "No prototype called 'A' found. Module 'A' is no prototype.".to_string(),
            Loc::new(48, 7, 5),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            DsgInvalidPrototypeAtSome,
            "No prototype called 'C' found.".to_string(),
            Loc::new(66, 7, 6),
            false,
        )
    );
}

#[test]
fn dsg2_some_chk_need_include() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D2_SomeChk_Include";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 2);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1);

    assert_eq!(
        *errs[0],
        Error::new_with_solution(
            DsgInvalidPrototypeAtSome,
            "No prototype called 'A' found.".to_string(),
            Loc::new(35, 7, 3),
            false,
            ErrorSolution::new("Try including 'Other'".to_string(), Loc::new(0, 1, 1))
        )
    );
}

#[test]
fn dsg3_impl_for_no_proto() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D3_Impl_ForNoProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            DsgProtoImplForNonProtoValue,
            "Cannot at a prototype implmentation block to a child of type 'B' that has no prototype components.".to_string(),
            Loc::new(98, 4, 11),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            DsgProtoImplForNonProtoValue,
            "Cannot at a prototype implmentation block to a child of type 'B' that has no prototype components.".to_string(),
            Loc::new(151, 4, 16),
            false,
        )
    );
}

#[test]
fn dsg3_impl_missing_field() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D3_Impl_MissingField.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3 + 3);

    assert_eq!(
        *errs[0],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'sub'.".to_string(),
            Loc::new(141, 4, 13),
            false,
        )
    );

    assert_eq!(
        *errs[1],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'sub2'.".to_string(),
            Loc::new(141, 4, 13),
            false,
        )
    );

    assert_eq!(
        *errs[2],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'sub2'.".to_string(),
            Loc::new(158, 4, 14),
            false,
        )
    );

    // NET

    assert_eq!(
        *errs[3],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'sub'.".to_string(),
            Loc::new(231, 4, 21),
            false,
        )
    );

    assert_eq!(
        *errs[4],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'sub2'.".to_string(),
            Loc::new(231, 4, 21),
            false,
        )
    );

    check_err!(
        *errs[5] =>
        DsgProtoImplMissingField,
        "Missing prototype impl field 'sub2'.",
        false,
        None
    );
}

#[test]
fn dsg3_impl_no_ty_or_include() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D3_Impl_NoTyOrInclude";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 2);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2 + 2);

    check_err!(
        *errs[0] =>
        DsgProtoImplTyMissing,
        "Unknown type 'B'.",
        false,
        Some(ErrorSolution::new("Try including 'Other'".to_string(), Loc::new(0, 1, 1)))
    );

    check_err!(
        *errs[1] =>
        DsgProtoImplTyMissing,
        "Unknown type 'C'.",
        false,
        None
    );

    // Net

    check_err!(
        *errs[2] =>
        DsgProtoImplTyMissing,
        "Unknown type 'B'.",
        false,
        Some(ErrorSolution::new("Try including 'Other'".to_string(), Loc::new(0, 1, 1)))
    );

    check_err!(
        *errs[3] =>
        DsgProtoImplTyMissing,
        "Unknown type 'C'.",
        false,
        None
    );
}

#[test]
fn dsg3_impl_assoc_not_proto() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D3_Impl_AssocNotProto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1 + 1);

    check_err!(
        *errs[0] =>
        DsgProtoImplAssociatedTyNotDerivedFromProto,
        "Assigned type 'X' does not fulfill the prototype 'A'.",
        false,
        None
    );

    // Net

    check_err!(
        *errs[1] =>
        DsgProtoImplAssociatedTyNotDerivedFromProto,
        "Assigned type 'X' does not fulfill the prototype 'A'.",
        false,
        None
    );
}

#[test]
fn dsg3_impl_no_impl() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/D3_Impl_NoImpl.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");
    assert_eq!(r.scopes.len(), 1);

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 1 + 1);

    check_err!(
        *errs[0] =>
        DsgProtoImlMissing,
        "Missing prototype impl block for type 'M'.",
        false,
        None
    );

    // Net

    check_err!(
        *errs[1] =>
        DsgProtoImlMissing,
        "Missing prototype impl block for type 'M'.",
        false,
        None
    );
}
