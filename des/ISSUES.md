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

# Load order of modules

The current load order at module creation is the following:

1. Create the ModuleContext - Thus module_id(), module_path(), parent(), ... are provided - not provided are child(), gate() (if gates are too be created)
2. Create the custom state using Module::new - This may use any module_specific function - we may want to provide gate() at least, child if possible (not possible currently)
3. Build NDL - This unlocks gate / and creates childs thus unlocking childs.

Step 2 is a bit problematic since it is not erognomic to not allready provide such information

Thus the order 1, 3, 2 would be ideal.
However at step 3 a ModuleRef is needed since Gates need a ModuleRefWeak as their owner.
Thus far unsolved issue.

Ideas:

- ModuleRefs should support unuinitalized customs states (this state must be behind the Arc so that the refs autmatically update once step 2 is performed)
- Allow non-owned gates and make a post-init step to create the gates in step 3 but initialized them only after custom state init

Note, that both versions should be implemented transparently using and Option.
use Option::unwrap_unchecked in all major calls to cirumvent performance hits.
Rational: custom state will only be used in two cases:

- after or at_sim_start -> thus custom state init allready done
- at Module::new then custom state load order is non-negotiable

# Unknown cost factor in cqueue_impl

As the flamegraph shows, there is an undifentiferd cost in cqueue_imlp::fetch_next that takes
as much time, as cqueue::fetch_next does (50 % of time for a total of 6 %)

This feactor is identified as memove with a big event set.
However 32byte events stes should have that big of a cost factor.
If they do .. figure out why
