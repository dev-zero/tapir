{
  description = "tapir - Rust native label printer interface";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "clippy" "rustfmt" ];
          targets = [ "aarch64-unknown-linux-gnu" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustToolchain
            pkgs.pkg-config
            pkgs.freetype
            pkgs.cargo-watch
            pkgs.cargo-nextest
            pkgs.unzip
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.systemdMinimal
            pkgs.usb-modeswitch
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk
            (pkgs.darwinMinVersionHook "11.0")
          ];

          shellHook = ''
            echo "tapir dev shell"
            echo "  $(rustc --version)"
            echo "  targets: aarch64-unknown-linux-gnu"
          '';
        };
      }
    );
}
