let
    nixpkgs = import common/nix/nixpkgs;

    rustChannel = nixpkgs.rustChannelOf {
        date = "2022-05-01";
        channel = "nightly";
        sha256 = "0yshryfh3n0fmsblna712bqgcra53q3wp1asznma1sf6iqrkrl02";
    };
in
    nixpkgs.mkShell {

        # Tools available in Nix shell.
        nativeBuildInputs = [
            nixpkgs.cacert
            nixpkgs.pkg-config
            nixpkgs.python3Packages.sphinx
            nixpkgs.spidermonkey_91.dev
            rustChannel.rust
        ];

    }
