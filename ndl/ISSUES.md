# ISSUES

## Perfomance cost after desugaring (001)

> After desugaring nodes get expanded and then typechecked
> this can lead to huge problems when using network-scale expansion

> unnessecary tych cause all expanded nodes / submodule share same type so tycheck
> must only be done one

> name collision checking harder. but shall also be done by using unexpanded macros.

## Add aliasing system and indirect generics

module A {
submodules:
app: some B
}

module B {}

module C instanceof B

# TODO(Cleanup): Use check_error macro in all examples

# Reorder dgs passes to use alias as a not proto impl module

# Internal restructuring (Poll based arch)

- central Ctx that caches results
- requests have dependencys

[GlobalTySpecCtx] depends on 'ScopeResolver' + foreach(scope) BuildSpec
[ScopeRsolver] depends on nothing
[BuildSpec] depends on 'ParsingResult' + 'GlobalTyDefCtx'
[ParsingResult] depends on 'TokenStream'
[TokenStream] depends on 'Asset'
[Asset] depends on nothing

[TyChk] passes are done internaly by the deps aboth

The central Ctx caches all results behind a std::rc::Arc

# Dsg internals

1. Resolve alias by copinign ModuleDef of prototype onto alias
2. Set derived from to check for compliance.
3. Desctructure into ModuleSpec
4. some / p-impl checking passes (read-only)
