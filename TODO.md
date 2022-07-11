# ndl - subsystem

Reuse network object as a generic subsystem object.
A subsystem contains nodes (either Modules or other Subsystems)
an can connect them via connections.

Additionally a subsystem can EXPOSE certain gates / gate clusters
of the contained nodes that are then treated as "gates of the subsystem"

In code a subsystem is a object that contains the following functions:

-   at_sim_start
-   at_sim_end
-   activity

It has handles to all nodes and by extension their connections
It also has handles to the exposed gates.

The nodes in the subsystem that are modules have no parent.
The subsytem structure may have parental information.

Subsystems can only be instantiated in [nodes] not [submodules].
Thereby every subsystem (that is actually used) is created by another subsystem.
In conclusion a tree is constructed. The subsystem ontop of this tree is declared
the runtime network. If mutipled non-connected tree exist pick one. Consider the other trees dead code.

The subsystem acts as a seperate component on the module path
[rt/subsys1/subsys2]/modue1/mod

[IMPL]

-   trait Subsystem {}
-   trait TopLevelSubsystem: Subsystem {}
-   #[derive(Subsystem)]

# NDL Warnings

e.g. mutiple import of the same subasset (in the same file+)

# Tokio investigate

18446744073709551615.999999999s
