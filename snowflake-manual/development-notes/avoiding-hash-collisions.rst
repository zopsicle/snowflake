========================
Avoiding hash collisions
========================

Snowflake uses cryptographic hashes for identifying cache entries.
It is of utmost important that different entries have different hashes.
An easy way to ensure this is to write the data to the hasher in such a way
that it can theoretically be parsed to reconstruct the original file
(i.e. if the hasher would remember the data written to it).

.. note::
   Code that produces hashes should mention this manual chapter.


Making the hash complete
------------------------

Use the following pattern when hashing structs:

.. code:: rust

   let Self{foo, bar, baz} = self;
   // ...

The Rust compiler will give an "unused variable" warning for each unused field.
It will also give an error if any fields are omitted from the pattern.
So it is no longer possible to accidentally omit fields from the hash.


Making the hash unambiguous
---------------------------

To make sure there are no disambiguities involving variable-length data,
we either prefix such data with a number indicating their length,
or we terminate it with a suitable sentinel value.
And if one of different types of values can be hashed,
each should be prefixed with a different discriminant.
