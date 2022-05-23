=================
Design principles
=================

The requirements surrounding the design of Sekka
are elaborated upon in this chapter.


Simple data model
-----------------

Sekka provides just a few built-in data types.
There is no method overriding or overloading.
The reasoning behind this is that if your problem is
complex enough that you need non-trivial custom data structures,
you shouldn't be using a dynamically typed programming language.
