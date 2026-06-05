{
  description = "labelmanagerpnp - Rust native label printer interface";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
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
            pkgs.cargo-watch
            pkgs.cargo-nextest
          ] ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
            pkgs.systemdMinimal
            pkgs.usb-modeswitch
          ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
            pkgs.apple-sdk
          ];

          shellHook = ''
            echo "labelmanagerpnp dev shell"
            echo "  $(rustc --version)"
            echo "  targets: aarch64-unknown-linux-gnu"
          '';
        };
      }
    );
}
