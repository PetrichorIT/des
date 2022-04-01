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

# Crash on empty files

# Check for cyclic types

# TODO(Cleanup): Use check_error macro in all examples
