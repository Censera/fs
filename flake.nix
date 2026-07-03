{
    description = "fsize";

    inputs = {
        nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
        flake-utils.url = "github:numtide/flake-utils";
        rust-overlay = {
            url = "github:oxalica/rust-overlay";
            inputs.nixpkgs.follows = "nixpkgs";
        };
    };

    outputs = { nixpkgs, flake-utils, rust-overlay, ... }:
    flake-utils.lib.eachDefaultSystem (system:
    let
    pkgs = import nixpkgs {
        inherit system;
        overlays = [ (import rust-overlay) ];
    };

    rustToolchain = pkgs.rust-bin.stable.latest.default.override {
        targets = [
            "x86_64-unknown-linux-musl"
            "aarch64-unknown-linux-gnu"
            "armv7-unknown-linux-gnueabihf"
            "riscv64gc-unknown-linux-gnu"
            "x86_64-pc-windows-gnu"
        ];

        extensions = [
            "rust-src"
            "rust-analyzer"
        ];
    };
    in {
        devShells.default = pkgs.mkShell {
            packages = [
                rustToolchain

                pkgs.pkg-config

                pkgs.pkgsCross.mingwW64.stdenv.cc
                pkgs.pkgsCross.aarch64-multiplatform.stdenv.cc
                pkgs.pkgsCross.armv7l-hf-multiplatform.stdenv.cc
                pkgs.pkgsCross.riscv64.stdenv.cc
            ];

            shellHook = ''
    echo "Done"
    '';
        };
    });
}
