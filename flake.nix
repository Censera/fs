{
    description = "fsize: file size tool";

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

    commonArgs = {
        pname = "fsize";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
            pkg-config
        ];
    };

    linuxPackage = pkgs.rustPlatform.buildRustPackage (commonArgs // {
        meta.mainProgram = "fsize";
    });

    windowsPackage =
    pkgs.pkgsCross.mingwW64.rustPlatform.buildRustPackage (commonArgs // {
        cargoBuildFlags = [ "--target=x86_64-pc-windows-gnu" ];

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER =
        "${pkgs.pkgsCross.mingwW64.stdenv.cc.targetPrefix}cc";
    });

    in {
        packages = {
            default = linuxPackage;
            fsize = linuxPackage;
            fsize-windows = windowsPackage;
        };

        devShells.default = pkgs.mkShell {
            packages = [
                rustToolchain

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
}{
    description = "fsize: file size tool";

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

    commonArgs = {
        pname = "fsize";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
            pkg-config
        ];
    };

    linuxPackage = pkgs.rustPlatform.buildRustPackage (commonArgs // {
        meta.mainProgram = "fsize";
    });

    windowsPackage =
    pkgs.pkgsCross.mingwW64.rustPlatform.buildRustPackage (commonArgs // {
        cargoBuildFlags = [ "--target=x86_64-pc-windows-gnu" ];

        CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER =
        "${pkgs.pkgsCross.mingwW64.stdenv.cc.targetPrefix}cc";
    });

    in {
        packages = {
            default = linuxPackage;
            fsize = linuxPackage;
            fsize-windows = windowsPackage;
        };

        devShells.default = pkgs.mkShell {
            packages = [
                rustToolchain

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
