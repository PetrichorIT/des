use des_net_utils::ndl::{
    def::{Def, ModuleGenericsDef, TypClause},
    error::ErrorKind,
    transform,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn comptime() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C2)
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                inherit: C
                gates:
                    - new
        "#,
    )?;

    let net = transform(&def)?;
    println!("================");
    println!("{}", serde_yml::to_string(&net)?);
    Ok(())
}

#[test]
fn typ_arguments_not_provided() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                inherit: C
                gates:
                    - new
        "#,
    )?;

    let err = transform(&def).unwrap_err();
    assert_eq!(
        err,
        ErrorKind::InvalidTypStatement(
            TypClause {
                ident: "B".to_string(),
                ..Default::default()
            },
            vec![ModuleGenericsDef {
                binding: "Inner".to_string(),
                bound: "C".to_string()
            }]
        )
    );
    Ok(())
}

#[test]
fn typ_arguments_wrong_count_provided() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C2, C2)
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                inherit: C
                gates:
                    - new
        "#,
    )?;

    let err = transform(&def).unwrap_err();
    assert_eq!(
        err,
        ErrorKind::InvalidTypStatement(
            TypClause {
                ident: "B".to_string(),
                args: vec!["C2".to_string(), "C2".to_string(),]
            },
            vec![ModuleGenericsDef {
                binding: "Inner".to_string(),
                bound: "C".to_string()
            }]
        )
    );
    Ok(())
}

#[test]
fn typ_arguments_no_interface_compliance() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C2)
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                gates:
                    - new
        "#,
    )?;

    let err = transform(&def).unwrap_err();
    assert_eq!(
        err,
        ErrorKind::AssignedTypDoesNotConformToInterface(TypClause {
            ident: "B".to_string(),
            args: vec!["C2".to_string()]
        },)
    );
    Ok(())
}

#[test]
fn typ_arguments_interface_compliance_without_inherit() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C2)
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                gates:
                    - port
        "#,
    )?;

    let net = transform(&def)?;
    assert_eq!(&*net.submodules[0].typ.submodules[0].typ.typ, "C2");
    Ok(())
}

#[test]
fn typ_arguments_interface_compliance_through_inherit() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C2)
            B(Inner <- C):
                submodules:
                    c: Inner
            C:
                gates:
                    - port
            C2:
                inherit: C
        "#,
    )?;

    let net = transform(&def)?;
    assert_eq!(&*net.submodules[0].typ.submodules[0].typ.typ, "C2");
    Ok(())
}

#[test]
fn typ_definiton_generics_already_defined() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
            B(Inner <- C, Inner <- C):
                submodules:
                    c: Inner
            C:
        "#,
    )?;

    let err = transform(&def).unwrap_err();
    assert_eq!(
        err,
        ErrorKind::SymbolAlreadyDefined("Inner <- C".to_string())
    );
    Ok(())
}

#[test]
fn typ_definition_disallow_generic_interface_by_other_generic() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
            B(Inner <- C, Inner2 <- Inner):
                submodules:
                    c: Inner
            C:
        "#,
    )?;

    let err = transform(&def).unwrap_err();
    assert_eq!(
        err,
        ErrorKind::UnresolvableDependency(vec!["B".to_string()])
    );
    Ok(())
}

#[test]
fn typ_definition_can_override_external_module() -> Result<()> {
    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C)
            B(C <- C):
                submodules:
                    c: C
            C:
        "#,
    )?;

    let net = transform(&def)?;
    assert_eq!(&*net.submodules[0].typ.submodules[0].typ.typ, "C");

    let def: Def = serde_yml::from_str(
        r#"
        entry: A
        modules:
            A:
                submodules:
                    b: B(C)
            B(D <- C):
                submodules:
                    c: D
            C:
            D:
                gates:
                    - d
        "#,
    )?;

    let net = transform(&def)?;
    assert_eq!(&*net.submodules[0].typ.submodules[0].typ.typ, "C");
    assert_eq!(net.submodules[0].typ.submodules[0].typ.gates.len(), 0);
    Ok(())
}
