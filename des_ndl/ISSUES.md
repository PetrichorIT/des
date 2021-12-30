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


