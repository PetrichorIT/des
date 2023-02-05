# Syntax parsing

## Keywords

-   include
-   link
-   entry
-   module
-   gates
-   submodules
-   connections

## Include statement

include<keyword> path/to/file<linkpath> ;<option<semi>>

# Link statement

link<keyword> ident<ident> [ :<colon> symbol<ident> ]<optional> {<delim>
key<ident> :<colon> 123<lit> ,<comma>
other<ident> :<colon> "str"<lit> ,<trailing_comma>
}<delim>

# Entry statement

entry<keywor> symbol<ident> ;<option<semi>>
