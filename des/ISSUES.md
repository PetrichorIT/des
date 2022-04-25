# Change derive structure (idea)

instead of generating code for the ndl impls,
generate only:

> function signature with associated proto-impl generics
> call to a generic load_spec function
> static module-core imps & co

write to a buildfile in the target dir;

> module spec in a efficent format
> call path of the current struct to prevent nessecity of imports
> entry in a call-independed org-file

on build:

> load org-file into runtime
> load associated increments into runtime
> at build use increments to build and connect submodules & gates

        like before.

issues:
how to make independent exectables that do not require a org-file at oad

# Alternative

instead of generating code:
write spec as const in the local scope
and write reusable fn to parse and apply consts

--> actual "code" size independet of spec
