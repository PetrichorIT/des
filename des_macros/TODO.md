# Build targeted resolver (Path A)

-   initalized with a workspace AND a target file (maybe derived by file_path!())
-   will lex and parse only the given target file and decide the lexing and parsing
    of other files based on the includes.
