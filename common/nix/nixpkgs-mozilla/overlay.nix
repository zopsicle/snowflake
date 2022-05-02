let
    tarball = fetchTarball (fromTOML (builtins.readFile ./pinned.toml));
in
    import tarball
