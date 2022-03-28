# Massive somthing dump at end of macro invocation

Compiling tests v0.1.0 (/home/petrichor/Developer/rust/des/tests)
error[45]: No local gate cluster 'netOut' exists on module 'Alice'.
--> tests/ndl/Main.ndl:18
| netOut --> Link --> child/netIn
18 | netIn <-- Link <-- child/netOut
| }
|
|
error[18]: Unexpected token. Expected closing bracket.
--> tests/ndl/Main.ndl:24
| netIn[3]
24 | netOut[3
| }
= Try adding ']'
in tests/ndl/Main.ndl:24
{"message":"Some NDL error occured","code":null,"level":"error","spans":[{"file_name":"tests/ndl/members.rs","byte_start":100,"byte_end":106,"line_start":8,"line_end":8,"column_start":17,"column_end":23,"is_primary":true,"text":[{"text":"#[derive(Debug, Module)]","highlight_start":17,"highlight_end":23}],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":{"span":{"file_name":"tests/ndl/members.rs","byte_start":100,"byte_end":106,"line_start":8,"line_end":8,"column_start":17,"column_end":23,"is_primary":false,"text":[{"text":"#[derive(Debug, Module)]","highlight_start":17,"highlight_end":23}],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null},"macro_decl_name":"#[derive(Module)]","def_site_span":{"file_name":"/home/petrichor/Developer/rust/des/des_derive/src/lib.rs","byte_start":1541,"byte_end":1596,"line_start":44,"line_end":44,"column_start":1,"column_end":56,"is_primary":false,"text":[{"text":"pub fn derive_module(input: TokenStream) -> TokenStream {","highlight_start":1,"highlight_end":56}],"label":null,"suggested_replacement":null,"suggestion_applicability":null,"expansion":null}}}],"children":[],"rendered":"\u001b[0m\u001b[1m\u001b[38;5;9merror\u001b[0m\u001b[0m\u001b[1m: Some NDL error occured\u001b[0m\n\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m--> \u001b[0m\u001b[0mtests/ndl/members.rs:8:17\u001b[0m\n\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m|\u001b[0m\n\u001b[0m\u001b[1m\u001b[38;5;12m8\u001b[0m\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m| \u001b[0m\u001b[0m#[derive(Debug, Module)]\u001b[0m\n\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m| \u001b[0m\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;9m^^^^^^\u001b[0m\n\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m|\u001b[0m\n\u001b[0m \u001b[0m\u001b[0m\u001b[1m\u001b[38;5;12m= \u001b[0m\u001b[0m\u001b[1mnote\u001b[0m\u001b[0m: this error originates in the derive macro `Module` (in Nightly builds, run with -Z macro-backtrace for more info)\u001b[0m\n\n"}
error: Some NDL error occured
--> tests/ndl/members.rs:29:10
|
29 | #[derive(Module, Debug)]
| ^^^^^^
|
= note: this error originates in the derive macro `Module` (in Nightly builds, run with -Z macro-backtrace for more info)

error: could not compile `tests` due to previous error
