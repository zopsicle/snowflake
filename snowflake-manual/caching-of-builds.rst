=================
Caching of builds
=================

Snowflake maintains two caches: an action cache and an output cache.
Both caches use cryptographic hashes as their cache keys.
The output cache stores outputs of actions in a content-addressed way;
each entry of the output cache is named after the hash of that entry.
The action cache maps actions' build hashes—hashes of
(configuration, dependencies) pairs—to entries of the output cache.
