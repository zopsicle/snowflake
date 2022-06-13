=================
Design principles
=================


Biozheff is dynamically typed
-----------------------------

The primary purpose of Biozheff is to be used as
the configuration language for the Snowflake build system.
This use case makes dynamic typing particularly attractive
when compared to the more quality-assuring static typing:

1. Biozheff programs will be compiled and run on every build.
   Static type checking requires algorithms of non-trivial complexity,
   and waiting for these during an interactive session is frustrating.

2. Build configurations do not run in production environments
   and aren't touched nearly as often as the built sources are,
   so the maintenance burden of dynamic typing is less of an issue.

3. The performance implications of dynamic typing are less of an issue,
   because build configurations are rather trivial computations.


Programs can be run in parallel
-------------------------------

In Snowflake, the configuration step is independent from the build step.
This means that no other tasks occupy CPU cores during the configuration step.
It is therefore beneficial for Biozheff to support parallel evaluation.
Parallelism does not have to be applicable to individual expressions;
it is sufficient to allow separate files to be evaluated in parallel.


Simple object model
-------------------

There is no need for the programmer to be able to
customize every single aspect of the object model.
Monkey patching, sophisticated reflection features,
customizable method lookup, etc are all out of scope.
(In fact, they should be out of scope in any language,
because all they do is make code slow and unmaintainable.)
