=================
Design principles
=================

The design of Snowflake is heavily inspired by that of `Bazel`_.
The requirements surrounding the design of Snowflake
are elaborated upon in this chapter.


Snowflake is language-agnostic
------------------------------

Snowflake does not attach special meanings to the contents of source files.
It can invoke any tool that can be run as a batch command.
The user can configure how Snowflake invokes those tools using a DSL.


Cache keys are complete
-----------------------

A cache key is *complete* if it fully describes the item being cached,
as opposed to a *partial* cache key that results in a corrupted cache
when the item being cached changes in ways not described by the cache key.
Each action is identified by the hash of everything [#partial]_ that is
observable by the command producing the outputs of the action.
This includes the command itself, any environment variables it is passed,
the hashes of the dependencies of the action, the CPU architecture, and so on.


Commands are containerized
--------------------------

Relying on the user to ensure cache keys are complete is bad for two reasons:

1. The cache key may depend on a large number of inputs
   and it is not practical for a human to analyze this correctly.

2. Due to the constant fear of corrupted caches,
   the user will be inclined to perform *clean builds*,
   which have a 0% cache hit rate and are therefore inefficient.

To increase assurance that the cache key is complete,
commands are run in individual containerized environments.
For instance, an action that does not declare a particular file as an input
must not be able to access that file to cause a partial cache key.
The Nix store is made available inside the containers,
but this is safe since Nix store paths are input-addressable.
Snowflake implements the container machinery itself.
There is no need for the user to install a container runtime.


Dependencies are granular
-------------------------

An action can depend on individual outputs of another action,
without depending on other outputs of that action.
This increases the cache hit rate when only
one output of a multi-output action changes.


Uniform interface for warnings-as-errors
----------------------------------------

Many compilers can emit *warnings* in addition to errors.
Unlike an error, a warning does not cause compilation to fail.
When shipping code, it must be devoid of warnings,
for which many compilers expose a flag such as ``-Werror``.
However, enforcing this behavior *during development* is undesirable;
merely commenting out a line of code could result in a cascade of warnings.
It should only be enforced in :abbr:`CI (continuous integration)`
or when otherwise compiling code to be shipped.

We shift the responsibility of treating warnings as errors
from individual compilers to the build system as a whole.
Each action may specify a regular expression for warnings.
If this regular expression matches the compiler output,
then the action is considered to have emitted a warning.
If Snowflake is instructed by the user to treat warnings as errors,
actions considered to have emitted warnings will fail.
Otherwise, Snowflake caches the compiler output
and displays it—even if there is a cache hit—ensuring that
warnings remain visible in a edit–compile cycle workflow.


Snowflake does not manage third-party dependencies
--------------------------------------------------

This `Nix`_ package manager does an astounding job
at providing packages in a hermetic and reproducible way.
The Nixpkgs package repository provides a wide range of packages.
There is no need for Snowflake to replicate all of its behavior
and for users to then build all their dependencies with Snowflake.
Instead, Snowflake integrates nicely with Nix and
makes the Nix store available in its containers.


.. _Bazel: https://bazel.build
.. _Nix: https://nixos.org


.. rubric:: Footnotes

.. [#partial]
   Some exceptions may be tolerated for pragmatic reasons.
   For instance, the modification date of a file
   may be considered irrelevant for the cache key
   even through a compiler could theoretically
   produce different output depending on it.
