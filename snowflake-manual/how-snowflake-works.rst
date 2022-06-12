===================
How Snowflake works
===================


.. index::
   single: action
   single: action graph
   single: build
   single: dependency
   single: input
   single: output
   single: perform
   single: static file

Actions
-------

An action describes how to build some outputs given some inputs.
For instance, an action could take a source file as an input
and compile it into an object file, which would be an output.
An input can be a static file or an output of another action (a *dependency*).
By interpreting dependencies as edges, any collection of actions forms a graph.
Topologically sorting this action graph gives an order
in which the actions can be built.

Before an action can be built, all of its inputs must exist.
That is, any static file inputs must already exist,
which is the responsibility of the programmer;
and any dependency inputs must already be built,
which happens automatically by the build system.
Then, the action is *performed*, which means running its procedure.
After an action was performed, it is inserted into the action cache,
and any outputs it produced are inserted into the output cache.
Only after successful insertion to the caches, an action is considered *built*.


.. index::
   single: state directory
   see: .snowflake; state directory

State directory
---------------

The state directory, typically linked at ``.snowflake``,
stores any data that must persist across builds,
as well as temporary files used during building.


.. index::
   single: action cache

Action cache
''''''''''''

The action cache stores information about previously succeeded actions.
In the action cache, actions are identified by their hash,
which consists of the action's configuration and inputs.
Each action is mapped to the hashes of the outputs it produced.


.. index::
   single: output cache

Output cache
''''''''''''

The output cache is simply a content-addressed store of outputs.
Each output ever produced by an action is stored in the output cache,
identified by the hash of the output.

The output cache also stores build logs of successful actions.
Build logs are often identical across builds (and even actions),
so storing them content-addressed is efficient.
