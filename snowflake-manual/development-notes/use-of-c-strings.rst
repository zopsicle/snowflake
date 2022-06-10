================
Use of C strings
================

The Rust ``CStr`` and ``CString`` types are used throughout for paths.
The overwhelming use of paths is to pass them to system calls,
which expect their string arguments to be nul-terminated.
``Path`` and ``PathBuf`` values are not nul-terminated,
which requires additional overhead on every system call.
Since we also use the ``*at`` family of system calls,
it is very rare to actually have to manipulate paths.
