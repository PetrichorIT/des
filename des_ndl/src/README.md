# Compile process

## Lexing

1. Read the raw source
2. Tokenize source
3. Validate the token stream (if invalid symbols are found ... error)
4. Sanitize the token stream (reducable AND invalid tokens are removed)

Errors in this section are of two tyes.

-   Forbidden symbols: are stripped away ... compiling can procede
-   Invalid symbols: TODO will produce transient errros

## Parsing

Input: raw souce && sanitized / validated token stream
Output: ParsingResult with Loc Blocks for defs (no stream)

1. Will go into subsections to parse syntax complexes
2. If synatx error occures ErrorContext will be marked as transient until the
   parser returns to top valid token sequence.
3. Invalids Defs will not be added to result

If errors occurr:

-   Errors will be handled transient, meaning if a later error occuress concering
    the super module it will be associated with the syntax error

## Linking
