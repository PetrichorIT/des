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

# Hooks

A hook is a Fn(&mut Self, Message) -> Result<(), Message>.
Hooks will be executed when a message arrives at a node.
Hooks can be prioritized with a usize.
If a Hook retuns Ok() the message was consumed.
If not use the Err(\_) Variant to invoke the next hook.
If no hooks match, use the handle_message as the default hook.

SimContext handlers are hooks.
Shutdown is invoked using a hook.

# Bug in message builders

length is set to the header when calling content.
however while the length cannot be set directly in can be set indirectly using
header() leading to incorrect messages.

# Create custom Ref/RefMut structures for MREF and SREF

-   Currently a cast to a type using as_ref / as_mut
    removed the ctx information in the relevant ref
-   Try attaching ctx inforamtion to better controll calls of
    ctx specific actions
-   Note that the ctx should be drived based on the caller not the callie
