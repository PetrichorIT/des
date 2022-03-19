# ISSUES

## Perfomance cost after desugaring (001)

> After deugaring nodes get expanded and then typechecked
> this can lead to huge problems when using network-scale expansion

> unnessecary tych cause all expanded nodes / submodule share same type so tycheck
> must only be done one

> name collision checking harder. but shall also be done by using unexpanded macros.

## Lextest different on unix / win

> since win uses \r\n the pos / len of whitespace characters and thus their general formatting
> in test is faulty

> lex tests deactivated for now since lexer is stable
> but this should be fixed later

## Desugar ordering problem (Double sugar ordering)

If a module A is desugared first befor another module B and A makes connections
using a cluster of B in non-cluster connection mode, then the cluster in B will
not be desugared and thus cannot be derived

Additionally if the order is reversed the clustered approach cannot be used anymore

- Use Specs in connection setup after parsing own children to explicitly use desugared nodes
- Must use Defs for child module type information, but make checks for cluster vs non-cluster defs on incoming ConnectionDef
