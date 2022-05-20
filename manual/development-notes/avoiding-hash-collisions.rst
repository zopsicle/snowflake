========================
Avoiding hash collisions
========================

Snowflake uses cryptographic hashes for identifying cache entries.
It is of utmost important that different entries have different hashes.
An easy way to ensure this is to write the data to the hasher in such a way
that it can theoretically be parsed to reconstruct the original file
(i.e. if the hasher would remember the data written to it).
To make sure there are no disambiguities involving variable-length data,
we either prefix such data with a number indicating their length,
or we terminate it with a suitable sentinel value.
And if one of different types of values can be hashed,
each should be prefixed with a different discriminant.
Code that produces hashes should mention this manual chapter.
