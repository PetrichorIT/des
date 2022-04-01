use ndl::ErrorCode::*;
use ndl::*;

#[test]
fn base() {
    let mut resolver =
        NdlResolver::new("tests/prototype/base").expect("Failed to create resolver.");

    println!("{}", resolver);

    let _ = resolver.run();

    println!("{}", resolver);

    assert!(!resolver.ectx.has_errors())
}

#[test]
fn par_failed_network_as_proto() {
    let path = "tests/prototype/P_Network_Proto.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

    assert_eq!(*errs[0], Error::new(
            ParModuleInvalidSectionIdentifer,
            "Invalid subsection identifier 'nodes'. Possibilities are gates / submodules / connections / parameters.".to_string(),
            Loc::new(45, 5, 4),
            false,
        ));

    assert!(errs[1].transient);
    assert_eq!(errs[1].code, ParUnexpectedKeyword);
    assert!(errs[2].transient);
    assert_eq!(errs[2].code, ParUnexpectedKeyword);
}

#[test]
fn par_alias_no_ident() {
    let path = "tests/prototype/P_Alias_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");

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

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 3);

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

    assert!(errs[1].transient);
    assert!(errs[2].transient);
}

#[test]
fn par_pimpl_no_ident() {
    //
    // Error output sorting may reorder stdout
    //
    let path = "tests/prototype/P_Impl_NoIdent.ndl";
    let mut r = NdlResolver::quiet(path).expect("Test case file does not seem to exist");

    r.run().expect("Failed run");

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

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 4);

    // Case 1
    assert_eq!(
        *errs[0],
        Error::new(
            ParProtoImplInvalidIdent,
            "Unexpected token '123'. Expected type ident.".to_string(),
            Loc::new(165, 3, 14),
            false,
        )
    );

    assert_eq!(
        *errs[2],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'inner'.".to_string(),
            Loc::new(138, 4, 13),
            false,
        )
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
    assert_eq!(
        *errs[1],
        Error::new(
            ParProtoImplInvalidIdent,
            "Unexpected token '}'. Expected type ident.".to_string(),
            Loc::new(309, 1, 25),
            false,
        )
    );

    assert_eq!(
        *errs[3],
        Error::new(
            DsgProtoImplMissingField,
            "Missing prototype impl field 'inner'.".to_string(),
            Loc::new(251, 4, 22),
            false,
        )
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

    assert!(r.ectx.has_errors());

    let errs = r.ectx.all().collect::<Vec<&Error>>();
    assert_eq!(errs.len(), 2);

    assert_eq!(
        *errs[0],
        Error::new(
            ParNetworkDoesntAllowSome,
            "Unexpected keyword 'some'. This is not allowed on network definitions.".to_string(),
            Loc::new(50, 4, 5),
            false,
        )
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
