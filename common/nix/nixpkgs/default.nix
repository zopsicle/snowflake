let
    tarball = fetchTarball (fromTOML (builtins.readFile ./pinned.toml));
    overlays = [ (import ../nixpkgs-mozilla/overlay.nix) ];
in
    import tarball { inherit overlays; }
